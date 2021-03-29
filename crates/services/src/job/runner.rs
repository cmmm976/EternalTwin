use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::{future::Future, sync::Arc, time::Duration};

use etwin_core::job::{JobStore, StoredTask, StoredTaskState, TaskStatus, UpdateTaskError, UpdateTaskOptions};
use etwin_core::types::EtwinError;
use tokio::{sync, time::Instant};

use super::{erased::TaskError, JobContext, TaskRegistry, TaskStep};

/// Configuration for building a [`TaskRunner`].
pub struct TaskRunnerOptions {
  /// How frequently to check for new tasks if there is currently no pending task.
  /// If `None`, no checks will be done unless notified.
  pub store_poll_interval: Option<etwin_core::core::Duration>,
  /// The minimum delay between two task steps.
  pub rate_limit: etwin_core::core::Duration,
}

const STATUS_RUNNING: usize = 1;
const STATUS_PAUSED: usize = 2;
const STATUS_SHUTDOWN: usize = 3;

struct Shared<TyJobStore> {
  job_store: TyJobStore,
  task_registry: TaskRegistry,
  status: AtomicUsize,
  runner_notify: sync::Notify,
  shutdown_rx: sync::watch::Receiver<()>,
}

#[derive(Clone)]
pub struct TaskRunner<TyJobStore>(Arc<Shared<TyJobStore>>);

impl<TyJobStore> TaskRunner<TyJobStore>
where
  TyJobStore: JobStore,
{
  /// Builds a `[TaskRunner]` instance; the returned future must
  /// be spawned on a Tokio executor and run to completion.
  ///
  /// Only a single runner should be created for a given [`JobStore`].
  /// Multiple runners running in parallel won't process tasks faster and
  /// will instead execute each task step several times.
  pub fn start(
    job_store: TyJobStore,
    task_registry: TaskRegistry,
    options: &TaskRunnerOptions,
  ) -> (Self, impl Future<Output = ()>) {
    let (shutdown_tx, shutdown_rx) = sync::watch::channel(());

    let shared = Arc::new(Shared {
      job_store,
      task_registry,
      status: AtomicUsize::new(STATUS_RUNNING),
      runner_notify: sync::Notify::new(),
      shutdown_rx,
    });

    let state = State::new(shared.clone(), shutdown_tx, options);
    (Self(shared), state.run())
  }

  pub fn registry(&self) -> &TaskRegistry {
    &self.0.task_registry
  }

  pub(super) fn raw_store(&self) -> &TyJobStore {
    &self.0.job_store
  }

  /// Submits a [`Task`] for execution. If the submitted task wasn't created from
  /// the [`TaskRegistry`] associated with this runner, the behavior is unspecified.
  pub async fn submit(&self, task: StoredTaskState) -> Result<StoredTask, EtwinError> {
    let state = task;
    let stored = self.0.job_store.create_task(&state, None).await?;

    // Wake up the runner task if it is running.
    if self.0.status.load(SeqCst) == STATUS_RUNNING {
      self.0.runner_notify.notify_one();
    }

    Ok(StoredTask { short: stored, state })
  }

  pub fn pause(&self) {
    // Pause the runner; no need to notify here: if it is waiting it will see the change on waking up.
    let _ = self
      .0
      .status
      .compare_exchange(STATUS_RUNNING, STATUS_PAUSED, SeqCst, SeqCst);
  }

  pub fn unpause(&self) {
    // Unpause the runner and wake it up.
    if self
      .0
      .status
      .compare_exchange(STATUS_PAUSED, STATUS_RUNNING, SeqCst, SeqCst)
      .is_ok()
    {
      self.0.runner_notify.notify_one();
    }
  }

  /// Gracefully shutdowns the runner, letting any running task step finish.
  ///
  /// The returned future can be used to wait for the runner's shutdown.
  pub fn shutdown(&self) -> impl Future<Output = ()> {
    self.0.status.store(STATUS_SHUTDOWN, SeqCst);
    // Wake up the runner task so that it can shutdown immediately.
    self.0.runner_notify.notify_one();

    let mut rx = self.0.shutdown_rx.clone();
    async move {
      // Wait for the sender half to close.
      rx.changed().await.unwrap_err();
    }
  }
}

struct State<TyJobStore> {
  runner: Arc<Shared<TyJobStore>>,
  // Will be dropped to signal the runner's shutdown; no value will be ever sent.
  _shutdown_tx: sync::watch::Sender<()>,
  next_task_start: Instant,
  store_poll_interval: Option<Duration>,
  rate_limit: Duration,
}

impl<TyJobStore: JobStore> State<TyJobStore> {
  fn new(runner: Arc<Shared<TyJobStore>>, shutdown_tx: sync::watch::Sender<()>, options: &TaskRunnerOptions) -> Self {
    Self {
      runner,
      _shutdown_tx: shutdown_tx,
      next_task_start: Instant::now(),
      store_poll_interval: options
        .store_poll_interval
        .map(|d| d.to_std().expect("poll interval cannot be negative")),
      rate_limit: options
        .rate_limit
        .to_std()
        .expect("rate limit interval cannot be negative"),
    }
  }

  async fn run(mut self) {
    let mut is_store_empty = false;
    loop {
      self.wait(is_store_empty).await;

      match self.runner.status.load(SeqCst) {
        STATUS_RUNNING => (),
        STATUS_SHUTDOWN => break,
        STATUS_PAUSED => self.runner.runner_notify.notified().await,
        s => unreachable!("invalid runner status: {}", s),
      }

      is_store_empty = match self.runner.job_store.get_next_task_to_run().await {
        Ok(Some(task)) => {
          if let Err(err) = self.step_task(task).await {
            // TODO: log and continue instead of panicking.
            panic!("TaskRunner couldn't save task state in store: {}", err);
          }
          false
        }
        Ok(None) => true,
        // TODO: log and continue instead of panicking.
        Err(err) => panic!("TaskRunner couldn't retrieve next task from store: {}", err),
      };
    }
  }

  async fn wait(&mut self, is_store_empty: bool) {
    let mut now = Instant::now();

    if is_store_empty {
      // If there is no pending tasks, sleep until we timeout or a notification happens.
      let poll_interval = self.store_poll_interval;
      let notify = self.runner.runner_notify.notified();

      if let Some(timeout) = poll_interval {
        let _ = tokio::time::timeout_at(now + timeout, notify).await;
      } else {
        notify.await;
      }
      now = Instant::now();
    }

    // Enforce rate-limiting.
    if now < self.next_task_start {
      tokio::time::sleep_until(self.next_task_start).await;
      now = Instant::now();
    }

    self.next_task_start = now.max(self.next_task_start + self.rate_limit);
  }

  async fn step_task(&self, mut task: StoredTask) -> Result<(), EtwinError> {
    use etwin_core::core::Duration;

    let jcx = JobContext {
      registry: &self.runner.task_registry,
    };

    let before = tokio::time::Instant::now();
    let children = execute_task_step(jcx, &mut task).await;
    let elapsed = tokio::time::Instant::now() - before;

    let id = task.short.id;

    // Spawn each child task in the job_store
    futures::future::try_join_all(
      children
        .into_iter()
        .map(|c| async move { self.runner.job_store.create_task(&c, Some(id)).await }),
    )
    .await?;

    let update = UpdateTaskOptions {
      id,
      current_step: task.short.step_count,
      step_time: Duration::from_std(elapsed).ok().unwrap_or_else(Duration::max_value),
      status: task.short.status,
      status_message: task.short.status_message.as_deref(),
      state: &task.state.state,
    };

    match self.runner.job_store.update_task(&update).await {
      Ok(_) => Ok(()),
      Err(UpdateTaskError::StepConflict { .. }) => {
        // Someone else updated the task state; do nothing.
        Ok(())
      }
      Err(err @ UpdateTaskError::InvalidTransition { .. }) | Err(err @ UpdateTaskError::NotFound(_)) => {
        // Somebody modified the task data under our feet, or we asked for the impossible.
        Err(err.into())
      }
      Err(UpdateTaskError::Other(err)) => {
        // Bubble up other errors.
        Err(err)
      }
    }
  }
}

async fn execute_task_step(jcx: JobContext<'_>, stored_task: &mut StoredTask) -> Vec<StoredTaskState> {
  assert_eq!(stored_task.short.status, TaskStatus::Running);

  let (status, msg, children) = jcx
    .registry
    .advance_stored(jcx, &stored_task.state)
    .await
    .and_then(|(step, state)| {
      let res = match step {
        TaskStep::Progress => (TaskStatus::Running, None, Vec::new()),
        TaskStep::Complete => (TaskStatus::Complete, None, Vec::new()),
        TaskStep::WaitFor(children) => {
          let children = children
            .iter()
            .map(|opts| jcx.registry.create_task_stored(opts))
            .collect::<Result<Vec<_>, _>>()?;

          (TaskStatus::Running, None, children)
        }
      };
      stored_task.state.state = state;
      Ok(res)
    })
    .unwrap_or_else(|err| (TaskStatus::Failed, Some(format_task_error_message(err)), Vec::new()));

  stored_task.short.status = status;
  stored_task.short.status_message = msg;

  children
}

fn format_task_error_message(err: TaskError) -> String {
  // TODO: show full error source chain in message
  err.to_string()
}

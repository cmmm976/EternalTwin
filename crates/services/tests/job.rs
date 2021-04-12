use std::sync::Arc;
use std::{cell::Cell, future::Future, pin::Pin, time::Duration};

use async_trait::async_trait;
use etwin_core::types::EtwinError;
use etwin_core::{
  auth::{AuthContext, AuthScope, UserAuthContext},
  clock::{Clock, SystemClock},
  job::{JobStore, OpaqueTaskData},
  user::{ShortUser, UserDisplayNameVersion, UserDisplayNameVersions},
  uuid::{Uuid4Generator, UuidGenerator},
};
use etwin_job_store::MemJobStore;
use serde::{Deserialize, Serialize};

use etwin_services::job::{JobService, TaskKind, TaskRegistry, TaskRunner, TaskRunnerOptions, TaskStep};

struct TestApi<TyClock> {
  _clock: Arc<TyClock>,
  _job_store: Arc<MemJobStore<Arc<dyn Clock>, Arc<dyn UuidGenerator>>>,
  task_runner: TaskRunner<Arc<dyn JobStore>>,
  job_service: JobService<Arc<dyn JobStore>>,
  fut: Cell<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
}

impl<TyClock> TestApi<TyClock>
where
  TyClock: Clock + 'static,
{
  fn new(
    clock: TyClock,
    uuid_generator: impl UuidGenerator + 'static,
    registry: TaskRegistry,
    options: TaskRunnerOptions,
  ) -> Self {
    let clock = Arc::new(clock);
    let job_store = {
      let clock: Arc<dyn Clock> = clock.clone();
      let uuid_generator: Arc<dyn UuidGenerator> = Arc::new(uuid_generator);
      Arc::new(MemJobStore::new(clock, uuid_generator))
    };
    let (task_runner, task) = TaskRunner::<Arc<dyn JobStore>>::start(job_store.clone(), registry, &options);
    let job_service = JobService::new(task_runner.clone());

    Self {
      _clock: clock,
      _job_store: job_store,
      task_runner,
      job_service,
      fut: Cell::new(Some(Box::pin(task))),
    }
  }

  async fn run_all_tasks<F: Future>(&self, timeout: Duration, task: F) -> F::Output {
    let fut = self.fut.take().expect("task runner is already started");

    let runner = tokio::task::spawn(fut);

    let fut = async move {
      let r = task.await;
      let _ = self.task_runner.shutdown();
      runner.await.expect("task runner panicked");
      r
    };

    tokio::time::timeout(timeout, fut).await.expect("test timeout exceeded")
  }
}

fn make_in_memory_api(registry: TaskRegistry) -> TestApi<SystemClock> {
  // We use the SystemClock so that tasks are executed in a realistic order.
  let clock = SystemClock;
  let uuid_generator = Uuid4Generator;
  let options = TaskRunnerOptions {
    store_poll_interval: None,
    rate_limit: etwin_core::core::Duration::zero(),
  };

  TestApi::new(clock, uuid_generator, registry, options)
}

fn make_admin_auth() -> AuthContext {
  AuthContext::User(UserAuthContext {
    scope: AuthScope::Default,
    user: ShortUser {
      id: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
      display_name: UserDisplayNameVersions {
        current: UserDisplayNameVersion {
          value: "Admin".parse().unwrap(),
        },
      },
    },
    is_administrator: true,
  })
}

#[tokio::test]
async fn test_many_tasks() {
  let (ping_tasks, mut ping_rx) = ping_task::create();

  let mut registry = TaskRegistry::new();
  registry.register(ping_tasks);
  registry.register(spawner_task::Kind);

  let api = make_in_memory_api(registry);
  let auth = make_admin_auth();

  api
    .run_all_tasks(Duration::from_secs(5), async {
      let task_options = r#"{
        "qty": 50,
        "options": { "pings": 25 }
      }"#;
      let task_options = OpaqueTaskData::from_string(task_options.into()).unwrap();

      for _ in 0..20 {
        api
          .job_service
          .create_job(&auth, spawner_task::Kind::NAME, &task_options)
          .await
          .unwrap();
      }

      for _ in 0..(20 * 50 * 25) {
        assert!(ping_rx.recv().await.is_some());
      }
    })
    .await;

  drop(api);

  let mut extra_pings = 0;
  while ping_rx.recv().await.is_some() {
    extra_pings += 1;
  }
  assert_eq!(extra_pings, 0);
}

mod spawner_task {
  use super::*;
  use etwin_services::job::JobContext;
  #[derive(Serialize, Deserialize)]
  pub struct Options {
    qty: usize,
    options: ping_task::Options,
  }

  pub struct Kind;

  #[async_trait]
  impl TaskKind for Kind {
    type Options = Options;
    type State = bool;

    const DATA_VERSION: u32 = 1;
    const NAME: &'static str = "Spawner";

    fn create(&self, _options: &Self::Options) -> Result<Self::State, EtwinError> {
      Ok(false)
    }

    async fn advance(
      &self,
      _jcx: JobContext<'_>,
      options: &Self::Options,
      state: &mut Self::State,
    ) -> Result<TaskStep, EtwinError> {
      if *state {
        Ok(TaskStep::Complete)
      } else {
        *state = true;
        Ok(TaskStep::WaitFor(
          (0..options.qty)
            .map(|_| ping_task::Kind::new_task(options.options))
            .collect(),
        ))
      }
    }
  }
}

mod ping_task {
  use super::*;
  use etwin_services::job::JobContext;
  use tokio::sync::mpsc;

  pub fn create() -> (Kind, mpsc::UnboundedReceiver<()>) {
    let (rx, tx) = mpsc::unbounded_channel();
    (Kind(rx), tx)
  }

  #[derive(Copy, Clone, Serialize, Deserialize)]
  pub struct Options {
    pings: usize,
  }

  pub struct Kind(mpsc::UnboundedSender<()>);

  #[async_trait]
  impl TaskKind for Kind {
    type Options = Options;
    type State = usize;

    const DATA_VERSION: u32 = 1;
    const NAME: &'static str = "Ping";

    fn create(&self, _options: &Self::Options) -> Result<Self::State, EtwinError> {
      Ok(0)
    }

    async fn advance(
      &self,
      _jcx: JobContext<'_>,
      options: &Self::Options,
      state: &mut Self::State,
    ) -> Result<TaskStep, EtwinError> {
      if *state < options.pings {
        *state += 1;
        self.0.send(())?;
        Ok(TaskStep::Progress)
      } else {
        Ok(TaskStep::Complete)
      }
    }
  }
}

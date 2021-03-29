use async_trait::async_trait;
use etwin_core::types::EtwinError;
use etwin_core::{
  clock::Clock,
  core::{Duration, Instant},
  job::{
    JobStore, ShortStoredTask, StoredTask, StoredTaskState, TaskId, TaskStatus, UpdateTaskError, UpdateTaskOptions,
  },
  uuid::UuidGenerator,
};
use std::collections::{BinaryHeap, HashMap};
use std::sync::RwLock;

// A task id with its priority.
// Tasks are ordered by decreasing time of last update.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TaskPriority {
  priority: std::cmp::Reverse<Instant>,
  id: TaskId,
}

struct StoreState {
  // All the stored tasks.
  tasks: HashMap<TaskId, StoredTask>,
  // Priority queue for currently running tasks.
  running: BinaryHeap<TaskPriority>,
  // Number of uncompleted children preventing tasks from running.
  blocking_children: HashMap<TaskId, u32>,
}

enum QueueEntryState {
  Valid,
  NeedsRemoval,
  NeedsUpdate(std::cmp::Reverse<Instant>),
}

impl StoreState {
  fn new() -> Self {
    Self {
      tasks: HashMap::new(),
      running: BinaryHeap::new(),
      blocking_children: HashMap::new(),
    }
  }

  fn peek_running_queue(&self) -> Option<(TaskId, QueueEntryState)> {
    let peek = self.running.peek()?;
    let id = peek.id;

    let entry = match self.tasks.get(&id) {
      Some(task) => {
        if task.short.status != TaskStatus::Running || self.blocking_children.contains_key(&id) {
          QueueEntryState::NeedsRemoval
        } else if peek.priority.0 != task.short.advanced_at {
          QueueEntryState::NeedsUpdate(std::cmp::Reverse(task.short.advanced_at))
        } else {
          QueueEntryState::Valid
        }
      }
      None => QueueEntryState::NeedsRemoval,
    };
    Some((id, entry))
  }

  fn peek_valid_task(&self) -> Option<&StoredTask> {
    let (task_id, state) = self.peek_running_queue()?;
    match (state, self.tasks.get(&task_id)) {
      (QueueEntryState::Valid, Some(task)) => Some(task),
      _ => panic!("no valid task on top of queue"),
    }
  }

  fn update_running_queue(&mut self) {
    while let Some((_, state)) = self.peek_running_queue() {
      match state {
        QueueEntryState::Valid => break,
        QueueEntryState::NeedsUpdate(p) => {
          self.running.peek_mut().unwrap().priority = p;
        }
        QueueEntryState::NeedsRemoval => {
          self.running.pop();
        }
      }
    }
  }

  fn add_task(&mut self, task: StoredTask) -> Result<(), EtwinError> {
    let id = task.short.id;
    let priority = std::cmp::Reverse(task.short.created_at);

    if let Some(parent) = task.short.parent {
      if !self.tasks.contains_key(&parent) {
        return Err(format!("Unknown task ID or parent: {}", parent).into());
      }
      let block = self.blocking_children.entry(parent).or_insert(0);
      *block = block.checked_add(1).expect("Blocking children overflow");
    }

    let old_task = self.tasks.insert(id, task);
    assert!(old_task.is_none());

    self.running.push(TaskPriority { id, priority });
    Ok(())
  }

  fn resolve_blocking_child(&mut self, parent: TaskId) {
    match self.blocking_children.get_mut(&parent) {
      None | Some(0) => panic!("No blocking children for task {}", parent),
      Some(children) => {
        *children -= 1;
        if *children == 0 {
          self.blocking_children.remove(&parent);
          self.running.push(TaskPriority {
            id: parent,
            priority: std::cmp::Reverse(self.tasks[&parent].short.advanced_at),
          })
        }
      }
    }
  }
}

pub struct MemJobStore<TyClock, TyUuidGenerator> {
  clock: TyClock,
  uuid_generator: TyUuidGenerator,
  state: RwLock<StoreState>,
}

impl<TyClock, TyUuidGenerator> MemJobStore<TyClock, TyUuidGenerator> {
  pub fn new(clock: TyClock, uuid_generator: TyUuidGenerator) -> Self {
    Self {
      clock,
      uuid_generator,
      state: RwLock::new(StoreState::new()),
    }
  }
}

#[async_trait]
impl<TyClock, TyUuidGenerator> JobStore for MemJobStore<TyClock, TyUuidGenerator>
where
  TyClock: Clock,
  TyUuidGenerator: UuidGenerator,
{
  async fn create_task(
    &self,
    task_state: &StoredTaskState,
    parent: Option<TaskId>,
  ) -> Result<ShortStoredTask, EtwinError> {
    let task_id = TaskId::from_uuid(self.uuid_generator.next());
    let start_time = self.clock.now();
    let short_task = ShortStoredTask {
      id: task_id,
      created_at: start_time,
      advanced_at: start_time,
      status: TaskStatus::Running,
      status_message: None,
      step_count: 0,
      running_time: Duration::zero(),
      parent,
    };

    let mut state = self.state.write().unwrap();
    state.add_task(StoredTask {
      short: short_task.clone(),
      state: task_state.clone(),
    })?;
    state.update_running_queue();

    Ok(short_task)
  }

  async fn update_task(&self, options: &UpdateTaskOptions<'_>) -> Result<ShortStoredTask, UpdateTaskError> {
    let mut state = self.state.write().unwrap();

    let stored_task = state
      .tasks
      .get_mut(&options.id)
      .ok_or(UpdateTaskError::NotFound(options.id))?;

    let task = &mut stored_task.short;

    if task.step_count != options.current_step {
      return Err(UpdateTaskError::StepConflict {
        task: task.id,
        actual: task.step_count,
        expected: options.current_step,
      });
    }

    if !task.status.can_transition_to(options.status) {
      return Err(UpdateTaskError::InvalidTransition {
        task: task.id,
        old: task.status,
        new: options.status,
      });
    }

    let next_step = task
      .step_count
      .checked_add(1)
      .ok_or_else(|| UpdateTaskError::Other("Task step count overflowed".into()))?;
    let running_time = task
      .running_time
      .checked_add(&options.step_time)
      .ok_or_else(|| UpdateTaskError::Other("Task running time overflowed".into()))?;

    task.advanced_at = self.clock.now();
    task.status = options.status;
    task.status_message = options.status_message.map(Into::into);
    task.running_time = running_time;
    task.step_count = next_step;
    stored_task.state.state = options.state.to_owned();

    let task = task.clone();

    if let Some(parent) = task.parent.filter(|_| options.status == TaskStatus::Complete) {
      state.resolve_blocking_child(parent);
    }

    state.update_running_queue();
    Ok(task)
  }

  async fn get_task(&self, task: TaskId) -> Result<Option<StoredTask>, EtwinError> {
    let state = self.state.read().unwrap();
    Ok(state.tasks.get(&task).cloned())
  }

  async fn get_next_task_to_run(&self) -> Result<Option<StoredTask>, EtwinError> {
    let state = self.state.read().unwrap();
    Ok(state.peek_valid_task().cloned())
  }
}

#[cfg(feature = "neon")]
impl<TyClock, TyUuidGenerator> neon::prelude::Finalize for MemJobStore<TyClock, TyUuidGenerator>
where
  TyClock: Clock,
  TyUuidGenerator: UuidGenerator,
{
}

#[cfg(test)]
mod test {
  use crate::mem::MemJobStore;
  use crate::test::TestApi;
  use chrono::{TimeZone, Utc};
  use etwin_core::{clock::VirtualClock, job::JobStore, uuid::Uuid4Generator};
  use std::sync::Arc;

  fn make_test_api() -> TestApi<Arc<VirtualClock>, Arc<dyn JobStore>> {
    let clock = Arc::new(VirtualClock::new(Utc.ymd(2020, 1, 1).and_hms(0, 0, 0)));
    let uuid_generator = Arc::new(Uuid4Generator);
    let job_store: Arc<dyn JobStore> = Arc::new(MemJobStore::new(Arc::clone(&clock), uuid_generator));

    TestApi { clock, job_store }
  }

  test_job_store!(|| make_test_api());
}

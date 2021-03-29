use std::borrow::Cow;

use crate::core::{Duration, Instant};
use crate::types::EtwinError;
use async_trait::async_trait;
use auto_impl::auto_impl;
use thiserror::Error;

#[cfg(feature = "_serde")]
use serde::{Deserialize, Serialize};

declare_new_uuid! {
  pub struct TaskId(Uuid);
  pub type ParseError = TaskIdParseError;
  const SQL_NAME = "etwin_task_id";
}

// TODO: add actual newtype, for better encapsulation
// This could also allow us to store a Box<dyn Serialize>.
pub type OpaqueTaskData = serde_json::value::RawValue;

declare_new_enum! {
  pub enum TaskStatus {
    #[str("Running")]
    Running,
    #[str("Complete")]
    Complete,
    #[str("Failed")]
    Failed,
    #[str("Stopped")]
    Stopped,
  }
  pub type ParseError = TaskStatusParseError;
  const SQL_NAME = "etwin_task_status";
}

impl TaskStatus {
  pub fn can_transition_to(self, other: Self) -> bool {
    use TaskStatus::*;
    match self {
      Running => matches!(other, Running | Complete | Failed | Stopped),
      Complete | Failed | Stopped => false,
    }
  }
}

#[cfg_attr(feature = "_serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShortStoredTask {
  pub id: TaskId,
  pub parent: Option<TaskId>,
  pub status: TaskStatus,
  pub status_message: Option<String>,
  pub created_at: Instant,
  pub advanced_at: Instant,
  pub step_count: u32,
  #[cfg_attr(
    feature = "_serde",
    serde(
      serialize_with = "etwin_serde_tools::duration_to_seconds_float",
      deserialize_with = "etwin_serde_tools::seconds_float_to_duration"
    )
  )]
  pub running_time: Duration,
}

#[cfg_attr(feature = "_serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct StoredTask {
  #[cfg_attr(feature = "_serde", serde(flatten))]
  pub short: ShortStoredTask,
  pub state: StoredTaskState,
}

#[cfg_attr(feature = "_serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct StoredTaskState {
  pub kind: Cow<'static, str>,
  pub data_version: u32,
  pub options: Box<OpaqueTaskData>,
  pub state: Box<OpaqueTaskData>,
}
#[derive(Error, Debug)]
pub enum UpdateTaskError {
  #[error("task {} not found", .0)]
  NotFound(TaskId),
  #[error("cannot update task {} (expected steps: {}, actual steps: {})", .task, .expected, .actual)]
  StepConflict { task: TaskId, expected: u32, actual: u32 },
  #[error("invalid transition for task {} ({} -> {})", .task, .old, .new)]
  InvalidTransition {
    task: TaskId,
    old: TaskStatus,
    new: TaskStatus,
  },
  #[error(transparent)]
  Other(EtwinError),
}

#[cfg_attr(feature = "_serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct UpdateTaskOptions<'a> {
  pub id: TaskId,
  pub current_step: u32,
  #[cfg_attr(
    feature = "_serde",
    serde(
      serialize_with = "etwin_serde_tools::duration_to_seconds_float",
      deserialize_with = "etwin_serde_tools::seconds_float_to_duration"
    )
  )]
  pub step_time: Duration,
  pub status: TaskStatus,
  pub status_message: Option<&'a str>,
  pub state: &'a OpaqueTaskData,
}

#[async_trait]
#[auto_impl(&, Arc)]
pub trait JobStore: Send + Sync {
  /// Creates a new task in the store, initially in the [`TaskStatus::Running`] state.
  /// If a `parent` task is specified, it won't run until the new task completes.
  async fn create_task(
    &self,
    task_state: &StoredTaskState,
    parent: Option<TaskId>,
  ) -> Result<ShortStoredTask, EtwinError>;

  /// Tries to update the state of an existing task.
  ///
  /// # Errors:
  /// - returns [`UpdateTaskError::NotFound`] if the task doesn't exist;
  /// - returns [`UpdateTaskError::StepConflict`] if the provider step number doesn't match the one in the store;
  /// - returns [`UpdateTaskError::InvalidTransition`] if the task cannot transition into the requested state.
  async fn update_task(&self, options: &UpdateTaskOptions<'_>) -> Result<ShortStoredTask, UpdateTaskError>;

  /// Retrieves the given task from the store, or [`None`] if it doesn't exist.
  async fn get_task(&self, task: TaskId) -> Result<Option<StoredTask>, EtwinError>;

  /// Retrieves the least recently updated task in the [`TaskStatus::Running`] state
  /// and whose children (if any) are all `Complete`d, or [`None`] if no such task exists.
  async fn get_next_task_to_run(&self) -> Result<Option<StoredTask>, EtwinError>;
}

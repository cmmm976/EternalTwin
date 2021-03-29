use super::{erased::TaskError, TaskOptions, TaskRegistry};
use async_trait::async_trait;
use etwin_core::types::EtwinError;
use serde::{Deserialize, Serialize};

pub enum TaskStep {
  Progress,
  Complete,
  WaitFor(Vec<TaskOptions>),
}

/// Serializable long-running [`Task`]s, for use in a [`TaskRunner`](super::TaskRunner).
#[async_trait]
pub trait TaskKind: Send + Sync + 'static {
  /// The name of this [`TaskKind`], used to identify it in its public API.
  /// Should be unique across the whole application.
  const NAME: &'static str;

  /// Initial options for creating new tasks. These are provided by the end user,
  /// and so constitute a part of the public API.
  ///
  /// Options cannot change during the lifecycle of a task instance, and
  /// as such should never make use of interior mutability.
  type Options: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static;

  /// The internal state of a task instance.
  type State: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static;

  /// Increment this number each time the serialized representation of
  /// [`Self::Options`] or [`Self::State`] changes.
  const DATA_VERSION: u32;

  /// Builds the initial state of a new task from its options.
  fn create(&self, options: &Self::Options) -> Result<Self::State, EtwinError>;

  /// Execute a step of a running task instance, updating its state.
  ///
  /// The updated state may be discarded if external failures occur, causing
  /// this method to be called multiple times with identical states.
  /// As such, any external side-effects should be idempotent.
  async fn advance(
    &self,
    jcx: JobContext<'_>,
    options: &Self::Options,
    state: &mut Self::State,
  ) -> Result<TaskStep, EtwinError>;

  /// Convenience method for creating a [`TaskOptions`] instance.
  fn new_task(options: Self::Options) -> TaskOptions {
    TaskOptions::new::<Self>(options)
  }

  /// Execute a task until completion.
  async fn run(&self, jcx: JobContext<'_>, options: &Self::Options) -> Result<(), TaskError> {
    let mut state = self.create(options).map_err(TaskError::InvalidOptions)?;
    loop {
      match self.advance(jcx, options, &mut state).await {
        Ok(TaskStep::Complete) => return Ok(()),
        Ok(TaskStep::Progress) => (),
        Ok(TaskStep::WaitFor(tasks)) => {
          let futs = tasks.into_iter().map(|opts| opts.run_task(jcx));
          futures::future::try_join_all(futs).await?;
        }
        Err(err) => return Err(TaskError::ExecFailed(err)),
      }
    }
  }
}

#[derive(Copy, Clone)]
pub struct JobContext<'cx> {
  pub(crate) registry: &'cx TaskRegistry,
}

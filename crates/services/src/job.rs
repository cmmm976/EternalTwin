use async_trait::async_trait;

use etwin_core::job::{JobId, JobStore, OpaqueTaskData, StoredTask, TaskId, TaskStatus, UpdateTaskOptions};
use etwin_core::types::EtwinError;
use etwin_core::{
  auth::{AuthContext, UserAuthContext},
  job::StoredJob,
};

mod erased;
mod runner;
mod task;

pub use erased::{TaskError, TaskOptions, TaskRegistry};
pub use runner::{TaskRunner, TaskRunnerOptions};
pub use task::{JobContext, TaskKind, TaskStep};

pub struct JobService<TyJobStore> {
  runner: TaskRunner<TyJobStore>,
}

impl<TyJobStore> JobService<TyJobStore>
where
  TyJobStore: JobStore,
{
  pub fn new(runner: TaskRunner<TyJobStore>) -> Self {
    Self { runner }
  }

  fn check_admin_auth(acx: &AuthContext) -> Result<(), EtwinError> {
    // TODO: this is ugly :c
    // Find a way to properly propagate a 403 HTTP error back.
    match acx {
      AuthContext::User(UserAuthContext {
        is_administrator: true, ..
      }) => Ok(()),
      _ => Err("JobService: NotEnoughAuthorization".into()),
    }
  }

  pub async fn create_job(
    &self,
    acx: &AuthContext,
    kind: &str,
    options: &OpaqueTaskData,
  ) -> Result<StoredTask, EtwinError> {
    Self::check_admin_auth(acx)?;

    let task = self.runner.registry().create_task_from_raw_stored(kind, options)?;
    self.runner.submit(task).await
  }

  pub async fn get_task(&self, acx: &AuthContext, task: TaskId) -> Result<Option<StoredTask>, EtwinError> {
    Self::check_admin_auth(acx)?;
    self.runner.raw_store().get_task(task).await
  }

  pub async fn get_job(&self, acx: &AuthContext, job: JobId) -> Result<Option<StoredJob>, EtwinError> {
    Self::check_admin_auth(acx)?;
    self.runner.raw_store().get_job(job).await
  }

  pub async fn stop_task(&self, acx: &AuthContext, task: TaskId) -> Result<Option<StoredTask>, EtwinError> {
    // Note: this won't pause any child tasks
    Self::check_admin_auth(acx)?;
    let store = self.runner.raw_store();
    let mut stored = match store.get_task(task).await? {
      None => return Ok(None),
      Some(j) => j, // TODO: should we return None if the task cannot be stopped?
    };

    let options = UpdateTaskOptions {
      id: task,
      current_step: stored.short.step_count,
      step_time: etwin_core::core::Duration::zero(),
      status: TaskStatus::Stopped,
      status_message: None,
      state: &stored.state.state,
    };
    stored.short = store.update_task(&options).await?;
    Ok(Some(stored))
  }

  pub async fn stop_job(&self, acx: &AuthContext, job: JobId) -> Result<(), EtwinError> {
    Self::check_admin_auth(acx)?;
    let store = self.runner.raw_store();
    store.update_job_status(job, TaskStatus::Stopped).await
  }

  pub fn pause_runner(&self, acx: &AuthContext) -> Result<(), EtwinError> {
    Self::check_admin_auth(acx)?;
    self.runner.pause();
    Ok(())
  }

  pub fn unpause_runner(&self, acx: &AuthContext) -> Result<(), EtwinError> {
    Self::check_admin_auth(acx)?;
    self.runner.unpause();
    Ok(())
  }
}

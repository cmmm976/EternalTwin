use std::{
  any::Any,
  borrow::Cow,
  collections::{hash_map::Entry, HashMap},
  marker::PhantomData,
};
use thiserror::Error;

use etwin_core::job::{OpaqueTaskData, StoredTaskState};
use etwin_core::types::EtwinError;

use super::*;

#[derive(Debug, Error)]
pub enum TaskError {
  #[error("Unknown task kind: {}", .0)]
  UnknownTaskKind(String),
  #[error("Invalid task options")]
  InvalidOptions(#[source] EtwinError),
  #[error("Failed to deserialize task")]
  LoadFailed(#[source] serde_json::Error),
  #[error("Invalid data version {} for task kind {}", .1, .0)]
  InvalidDataVersion(&'static str, u32),
  #[error("Failed to serialize task")]
  StoreFailed(#[source] serde_json::Error),
  #[error("Failed to execute task step")]
  ExecFailed(#[source] EtwinError),
}

pub struct TaskOptions(Box<dyn TaskOptionsDyn>);

impl TaskOptions {
  pub fn new<T: TaskKind + ?Sized>(options: T::Options) -> TaskOptions {
    Self(Box::new(TaskOptionsRepr {
      _marker: PhantomData::<T>,
      options,
    }))
  }

  pub async fn run_task(self, jcx: JobContext<'_>) -> Result<(), TaskError> {
    jcx.registry.run(jcx, &self).await
  }

  fn task_kind(&self) -> &'static str {
    self.0.task_kind()
  }

  fn downcast_options<T: TaskKind + ?Sized>(&self) -> Option<&T::Options> {
    Any::downcast_ref::<TaskOptionsRepr<T>>(self.0.as_any()).map(|o| &o.options)
  }
}

struct TaskOptionsRepr<T: TaskKind + ?Sized> {
  _marker: PhantomData<T>,
  options: T::Options,
}

#[async_trait]
trait TaskOptionsDyn: Any + Send + Sync {
  fn task_kind(&self) -> &'static str;

  fn as_any(&self) -> &dyn Any;
}

#[async_trait]
impl<T: TaskKind + ?Sized> TaskOptionsDyn for TaskOptionsRepr<T> {
  fn task_kind(&self) -> &'static str {
    T::NAME
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[async_trait]
trait TaskKindDyn: Send + Sync {
  fn create_stored_from_raw(&self, options: &OpaqueTaskData) -> Result<StoredTaskState, TaskError>;

  fn create_stored(&self, options: &TaskOptions) -> Result<StoredTaskState, TaskError>;

  async fn run(&self, jcx: JobContext<'_>, options: &TaskOptions) -> Result<(), TaskError>;

  async fn advance_stored(
    &self,
    jcx: JobContext<'_>,
    task: &StoredTaskState,
  ) -> Result<(TaskStep, Box<OpaqueTaskData>), TaskError>;
}

#[async_trait]
impl<T: TaskKind + 'static> TaskKindDyn for T {
  fn create_stored_from_raw(&self, options: &OpaqueTaskData) -> Result<StoredTaskState, TaskError> {
    create_stored_from_options(self, &deserialize_task_data(options)?)
  }

  fn create_stored(&self, options: &TaskOptions) -> Result<StoredTaskState, TaskError> {
    match options.downcast_options::<T>() {
      Some(opts) => create_stored_from_options(self, &opts),
      None => Err(TaskError::UnknownTaskKind(options.task_kind().into())),
    }
  }

  async fn run(&self, jcx: JobContext<'_>, options: &TaskOptions) -> Result<(), TaskError> {
    match options.downcast_options::<T>() {
      Some(opts) => self.run(jcx, opts).await,
      None => Err(TaskError::UnknownTaskKind(options.task_kind().into())),
    }
  }

  async fn advance_stored(
    &self,
    jcx: JobContext<'_>,
    task: &StoredTaskState,
  ) -> Result<(TaskStep, Box<OpaqueTaskData>), TaskError> {
    if task.data_version != T::DATA_VERSION {
      return Err(TaskError::InvalidDataVersion(T::NAME, task.data_version));
    }

    let options = deserialize_task_data(&task.options)?;
    let mut state = deserialize_task_data(&task.state)?;
    let step = self
      .advance(jcx, &options, &mut state)
      .await
      .map_err(TaskError::ExecFailed)?;
    let state = serialize_task_data(&state)?;

    Ok((step, state))
  }
}

fn create_stored_from_options<T: TaskKind + ?Sized>(
  kind: &T,
  options: &T::Options,
) -> Result<StoredTaskState, TaskError> {
  let state = kind.create(options).map_err(TaskError::InvalidOptions)?;
  Ok(StoredTaskState {
    kind: Cow::Borrowed(T::NAME),
    data_version: T::DATA_VERSION,
    options: serialize_task_data(&options)?,
    state: serialize_task_data(&state)?,
  })
}

fn serialize_task_data<T: serde::Serialize>(data: &T) -> Result<Box<OpaqueTaskData>, TaskError> {
  serde_json::value::to_raw_value(data).map_err(TaskError::StoreFailed)
}

fn deserialize_task_data<'de, T: serde::Deserialize<'de>>(data: &'de OpaqueTaskData) -> Result<T, TaskError> {
  serde_json::from_str(data.get()).map_err(TaskError::LoadFailed)
}

pub struct TaskRegistry {
  kinds: HashMap<&'static str, Box<dyn TaskKindDyn>>,
}

impl Default for TaskRegistry {
  fn default() -> Self {
    Self::new()
  }
}

impl TaskRegistry {
  pub fn new() -> Self {
    Self { kinds: HashMap::new() }
  }

  pub fn register<T: TaskKind + 'static>(&mut self, task_kind: T) {
    let name = T::NAME;
    match self.kinds.entry(name) {
      Entry::Occupied(_) => panic!("Duplicate task kind in registry: {:?}", name),
      Entry::Vacant(entry) => entry.insert(Box::new(task_kind)),
    };
  }

  pub(crate) fn create_task_from_raw_stored(
    &self,
    kind: &str,
    options: &OpaqueTaskData,
  ) -> Result<StoredTaskState, TaskError> {
    match self.kinds.get(kind) {
      Some(kind) => kind.create_stored_from_raw(options),
      None => Err(TaskError::UnknownTaskKind(kind.into())),
    }
  }

  pub(crate) fn create_task_stored(&self, options: &TaskOptions) -> Result<StoredTaskState, TaskError> {
    let kind = options.0.task_kind();
    match self.kinds.get(kind) {
      Some(kind) => kind.create_stored(options),
      None => Err(TaskError::UnknownTaskKind(kind.into())),
    }
  }

  pub(crate) async fn run(&self, jcx: JobContext<'_>, options: &TaskOptions) -> Result<(), TaskError> {
    let kind = options.0.task_kind();
    match self.kinds.get(kind) {
      Some(kind) => kind.run(jcx, options).await,
      None => Err(TaskError::UnknownTaskKind(kind.into())),
    }
  }

  pub(crate) async fn advance_stored<'cx>(
    &self,
    jcx: JobContext<'_>,
    task: &StoredTaskState,
  ) -> Result<(TaskStep, Box<OpaqueTaskData>), TaskError> {
    let kind_name: &str = &*task.kind;
    match self.kinds.get(kind_name) {
      Some(kind) => kind.advance_stored(jcx, task).await,
      None => Err(TaskError::UnknownTaskKind(kind_name.into())),
    }
  }
}

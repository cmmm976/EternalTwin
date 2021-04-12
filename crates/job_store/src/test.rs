use chrono::{Duration, TimeZone, Utc};
use etwin_core::job::{
  JobStore, OpaqueTaskData, ShortStoredTask, StoredJob, StoredTaskState, TaskId, TaskStatus, UpdateTaskError,
  UpdateTaskOptions,
};
use etwin_core::{api::ApiRef, clock::VirtualClock};
use uuid::Uuid;

macro_rules! test_job_store {
  ($(#[$meta:meta])* || $api:expr) => {
    register_test!($(#[$meta])*, $api, test_empty);
    register_test!($(#[$meta])*, $api, test_create_and_update_task);
    register_test!($(#[$meta])*, $api, test_invalid_task_updates);
    register_test!($(#[$meta])*, $api, test_task_run_order);
  };
}

macro_rules! register_test {
  ($(#[$meta:meta])*, $api:expr, $test_name:ident) => {
    #[tokio::test]
    $(#[$meta])*
    async fn $test_name() {
      crate::test::$test_name($api).await;
    }
  };
}

#[track_caller]
async fn assert_stored_task_eq<TyJobStore: JobStore>(
  store: &TyJobStore,
  task: &ShortStoredTask,
  state: &StoredTaskState,
) {
  let actual = store.get_task(task.id).await.unwrap().unwrap();

  assert_eq!(&actual.short, task);
  assert_eq!(actual.state.kind, state.kind);
  assert_eq!(actual.state.data_version, state.data_version);
  assert_eq!(actual.state.options.get(), state.options.get());
  assert_eq!(actual.state.state.get(), state.state.get());
}

pub(crate) struct TestApi<TyClock, TyJobStore>
where
  TyClock: ApiRef<VirtualClock>,
  TyJobStore: JobStore,
{
  pub(crate) clock: TyClock,
  pub(crate) job_store: TyJobStore,
}

pub(crate) async fn test_empty<TyClock, TyJobStore>(api: TestApi<TyClock, TyJobStore>)
where
  TyClock: ApiRef<VirtualClock>,
  TyJobStore: JobStore,
{
  let actual = api.job_store.get_next_task_to_run().await.unwrap();
  assert!(actual.is_none());

  let uuid = TaskId::from_uuid(Uuid::from_u128(0));
  let actual = api.job_store.get_task(uuid).await.unwrap();
  assert!(actual.is_none());
}

pub(crate) async fn test_create_and_update_task<TyClock, TyJobStore>(api: TestApi<TyClock, TyJobStore>)
where
  TyClock: ApiRef<VirtualClock>,
  TyJobStore: JobStore,
{
  api.clock.as_ref().advance_to(Utc.ymd(2021, 1, 1).and_hms(0, 0, 0));

  let mut task = StoredTaskState {
    kind: "frobnicator".into(),
    data_version: 42,
    options: OpaqueTaskData::from_string(r#"{"options": null}"#.into()).unwrap(),
    state: OpaqueTaskData::from_string(r#"{"state": null}"#.into()).unwrap(),
  };

  let initial = api.job_store.create_job(&task).await.unwrap();
  let expected = ShortStoredTask {
    id: initial.id,
    job_id: initial.job_id,
    parent: None,
    status: TaskStatus::Running,
    status_message: None,
    created_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 0),
    advanced_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 0),
    step_count: 0,
    running_time: Duration::zero(),
  };
  assert_eq!(initial, expected);
  assert_stored_task_eq(&api.job_store, &expected, &task).await;

  let actual_job = api.job_store.get_job(initial.job_id).await.unwrap();
  assert_eq!(
    actual_job,
    Some(StoredJob {
      id: initial.job_id,
      created_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 0),
      root_task: initial.id,
    })
  );

  api.clock.as_ref().advance_by(Duration::seconds(1));
  task.state = OpaqueTaskData::from_string(r#"{"more_state": 100}"#.into()).unwrap();
  let updated = api
    .job_store
    .update_task(&UpdateTaskOptions {
      id: initial.id,
      current_step: 0,
      step_time: Duration::milliseconds(500),
      status: TaskStatus::Running,
      status_message: None,
      state: &task.state,
    })
    .await
    .unwrap();
  let expected = ShortStoredTask {
    id: initial.id,
    job_id: initial.job_id,
    parent: None,
    status: TaskStatus::Running,
    status_message: None,
    created_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 0),
    advanced_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 1),
    step_count: 1,
    running_time: Duration::milliseconds(500),
  };
  assert_eq!(updated, expected);
  assert_stored_task_eq(&api.job_store, &expected, &task).await;

  api.clock.as_ref().advance_by(Duration::seconds(1));
  task.state = OpaqueTaskData::from_string(r#"{"done": "result"}"#.into()).unwrap();
  let finished = api
    .job_store
    .update_task(&UpdateTaskOptions {
      id: initial.id,
      current_step: 1,
      step_time: Duration::milliseconds(750),
      status: TaskStatus::Complete,
      status_message: Some("Completed!"),
      state: &task.state,
    })
    .await
    .unwrap();
  let expected = ShortStoredTask {
    id: initial.id,
    job_id: initial.job_id,
    parent: None,
    status: TaskStatus::Complete,
    status_message: Some("Completed!".into()),
    created_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 0),
    advanced_at: Utc.ymd(2021, 1, 1).and_hms(0, 0, 2),
    step_count: 2,
    running_time: Duration::milliseconds(1250),
  };
  assert_eq!(finished, expected);
  assert_stored_task_eq(&api.job_store, &finished, &task).await;
}

pub(crate) async fn test_invalid_task_updates<TyClock, TyJobStore>(api: TestApi<TyClock, TyJobStore>)
where
  TyClock: ApiRef<VirtualClock>,
  TyJobStore: JobStore,
{
  let simple_task_state = StoredTaskState {
    kind: "simple".into(),
    data_version: 0,
    options: OpaqueTaskData::from_string("null".into()).unwrap(),
    state: OpaqueTaskData::from_string("null".into()).unwrap(),
  };

  let unknown = TaskId::from_uuid(Uuid::from_u128(0));

  let make_update_options = |id: TaskId, status: TaskStatus, current_step: u32| UpdateTaskOptions {
    id,
    status,
    current_step,
    step_time: Duration::zero(),
    state: &simple_task_state.state,
    status_message: None,
  };

  macro_rules! assert_invalid {
    ($($args:expr),* => $err:pat) => {
      assert!(matches!(
        api
          .job_store
          .update_task(&make_update_options($($args),*))
          .await,
        Err($err)
      ));
    }
  }

  // Invalid parent.
  api
    .job_store
    .create_subtask(&simple_task_state, unknown)
    .await
    .unwrap_err();

  // Create a new task.
  let task = api.job_store.create_job(&simple_task_state).await.unwrap().id;

  assert_invalid!(unknown, TaskStatus::Running, 0 => UpdateTaskError::NotFound(_));
  assert_invalid!(task, TaskStatus::Running, 1 => UpdateTaskError::StepConflict { .. });

  // Mark the new task as completed.
  api
    .job_store
    .update_task(&make_update_options(task, TaskStatus::Complete, 0))
    .await
    .unwrap();

  assert_invalid!(task, TaskStatus::Running, 2 => UpdateTaskError::StepConflict { .. });
  assert_invalid!(task, TaskStatus::Running, 1 => UpdateTaskError::InvalidTransition { .. });
  assert_invalid!(task, TaskStatus::Failed, 1 => UpdateTaskError::InvalidTransition { .. });
  assert_invalid!(task, TaskStatus::Stopped, 1 => UpdateTaskError::InvalidTransition { .. });
}

pub(crate) async fn test_task_run_order<TyClock, TyJobStore>(api: TestApi<TyClock, TyJobStore>)
where
  TyClock: ApiRef<VirtualClock>,
  TyJobStore: JobStore,
{
  let mut simple_task_state = StoredTaskState {
    kind: "".into(),
    data_version: 0,
    options: OpaqueTaskData::from_string("null".into()).unwrap(),
    state: OpaqueTaskData::from_string("null".into()).unwrap(),
  };

  let advance_clock = || api.clock.as_ref().advance_by(Duration::seconds(1));

  macro_rules! make_tasks {
    ($(let $name:ident = $parent:expr;)*) => {
      $(
        simple_task_state.kind = stringify!($name).into();
        let $name = match $parent {
          None => api.job_store.create_job(&simple_task_state),
          Some(parent) =>  api.job_store.create_subtask(&simple_task_state, parent),
        }.await.unwrap().id;
        advance_clock();
      )*
    }
  }

  macro_rules! exec_tasks {
    ($($step:literal: $task_name:ident => $status:expr;)*) => {
      $(
        let task = api.job_store.get_next_task_to_run().await.unwrap().unwrap();
        if task.short.id != $task_name {
          panic!("step {}: expected task {}, got {}", $step, stringify!($task_name), task.state.kind);
        }
        api.job_store.update_task(&UpdateTaskOptions {
          id: task.short.id,
          status: $status,
          status_message: None,
          current_step: task.short.step_count,
          step_time: Duration::zero(),
          state: &simple_task_state.state,
        }).await.unwrap();
        advance_clock();
      )*
    }
  }

  make_tasks! {
    let task1 = None;
    let parent = None;
    let child1 = Some(parent);
    let task2 = None;
    let task3 = None;
    let child2 = Some(parent);
  }

  exec_tasks! {
    1: task1 => TaskStatus::Complete;
    2: child1 => TaskStatus::Running;
    3: task2 => TaskStatus::Running;
    4: task3 => TaskStatus::Running;
    5: child2 => TaskStatus::Running;

    6: child1 => TaskStatus::Running;
    7: task2 => TaskStatus::Complete;
    8: task3 => TaskStatus::Running;
    9: child2 => TaskStatus::Complete;

    10: child1 => TaskStatus::Complete;
    12: parent => TaskStatus::Running;
    11: task3 => TaskStatus::Running;

    12: parent => TaskStatus::Complete;
    13: task3 => TaskStatus::Complete;
  }

  assert!(api.job_store.get_next_task_to_run().await.unwrap().is_none());
}

// TODO: Tests for JobStore::update_job_status

use async_trait::async_trait;
use etwin_core::job::{
  JobStore, OpaqueTaskData, ShortStoredTask, StoredTask, StoredTaskState, TaskId, TaskStatus, UpdateTaskError,
  UpdateTaskOptions,
};
use etwin_core::{api::ApiRef, clock::Clock, types::EtwinError, uuid::UuidGenerator};
use etwin_core::{
  core::{Duration, Instant},
  job::{JobId, StoredJob},
};
use sqlx::postgres::types::PgInterval;
use sqlx::PgPool;
use std::{convert::TryFrom, error::Error};

type DbTransaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

pub struct PgJobStore<TyClock, TyDatabase, TyUuidGenerator> {
  clock: TyClock,
  database: TyDatabase,
  uuid_generator: TyUuidGenerator,
}

const DAYS_IN_MONTH: i64 = 30;

fn pg_interval_to_duration(interval: PgInterval) -> Result<Duration, sqlx::Error> {
  let max_days = Duration::max_value().num_days();
  let min_days = Duration::min_value().num_days();
  let days = interval.days as i64 + DAYS_IN_MONTH * interval.months as i64;
  let days = if days >= min_days && days <= max_days {
    Some(Duration::days(days)) // Can't overflow because of the above check.
  } else {
    None
  };
  let duration = days.and_then(|days| Duration::microseconds(interval.microseconds).checked_add(&days));
  duration.ok_or_else(|| {
    let err = Duration::from_std(std::time::Duration::from_secs(u64::MAX)).unwrap_err();
    sqlx::Error::Decode(err.into())
  })
}

fn duration_to_pg_interval(duration: Duration) -> Result<PgInterval, sqlx::Error> {
  let days = duration.num_days();
  let micros = (duration - Duration::days(days)).num_microseconds();
  let months = i32::try_from(days / DAYS_IN_MONTH);
  let days = (days % DAYS_IN_MONTH) as i32;

  match (micros, months) {
    (Some(microseconds), Ok(months)) => Ok(PgInterval {
      months,
      days,
      microseconds,
    }),
    _ => Err(sqlx::Error::Decode("Overflow while encoding duration".into())),
  }
}

#[derive(Debug, sqlx::FromRow)]
struct StoredTaskRow {
  etwin_task_id: TaskId,
  etwin_job_id: JobId,
  parent_task_id: Option<TaskId>,
  ctime: Instant,
  atime: Instant,
  status: TaskStatus,
  status_message: Option<String>,
  // Should be u32, but postgresql is being annoying :c
  step_count: i64,
  running_time: PgInterval,
  kind: String,
  // Should be u32, but postgresql is being annoying :c
  data_version: i64,
  options: String,
  state: String,
}

impl TryFrom<StoredTaskRow> for StoredTask {
  type Error = EtwinError;

  fn try_from(r: StoredTaskRow) -> Result<Self, Self::Error> {
    Ok(StoredTask {
      short: ShortStoredTask {
        id: r.etwin_task_id,
        job_id: r.etwin_job_id,
        parent: r.parent_task_id,
        created_at: r.ctime,
        advanced_at: r.atime,
        status: r.status,
        status_message: r.status_message,
        step_count: u32::try_from(r.step_count)?,
        running_time: pg_interval_to_duration(r.running_time)?,
      },
      state: StoredTaskState {
        kind: r.kind.into(),
        data_version: u32::try_from(r.data_version)?,
        options: OpaqueTaskData::from_string(r.options)?,
        state: OpaqueTaskData::from_string(r.state)?,
      },
    })
  }
}

async fn insert_task_raw(
  tx: &mut DbTransaction<'_>,
  start_time: Instant,
  id: TaskId,
  job_id: JobId,
  parent: Option<TaskId>,
  task_state: &StoredTaskState,
) -> Result<ShortStoredTask, EtwinError> {
  let task = ShortStoredTask {
    id,
    job_id,
    parent,
    created_at: start_time,
    advanced_at: start_time,
    status: TaskStatus::Running,
    status_message: None,
    step_count: 0,
    running_time: Duration::zero(),
  };

  sqlx::query(
    r"
      INSERT INTO etwin_tasks(
        etwin_task_id, ctime, atime, etwin_job_id, parent_task_id, status, status_message,
        step_count, running_time, kind, data_version, options, state, _blocking_children
      ) VALUES (
        $1::ETWIN_TASK_ID, $2::INSTANT, $2::INSTANT, $3::ETWIN_JOB_ID, $4::ETWIN_TASK_ID,
        $5::ETWIN_TASK_STATUS, NULL, 0, '0 seconds'::INTERVAL, $6::VARCHAR, $7::U32, $8::JSON, $9::JSON, 0
      );
    ",
  )
  .bind(&task.id)
  .bind(&start_time)
  .bind(&job_id)
  .bind(&parent)
  .bind(&task.status)
  .bind(&*task_state.kind)
  .bind(&task_state.data_version)
  .bind(task_state.options.get())
  .bind(task_state.state.get())
  .execute(&mut *tx)
  .await?;

  Ok(task)
}

impl<TyClock, TyDatabase, TyUuidGenerator> PgJobStore<TyClock, TyDatabase, TyUuidGenerator>
where
  TyClock: Clock,
  TyDatabase: ApiRef<PgPool>,
  TyUuidGenerator: UuidGenerator,
{
  pub async fn new(clock: TyClock, database: TyDatabase, uuid_generator: TyUuidGenerator) -> Result<Self, EtwinError> {
    // Check that we can connect to the database.
    let _ = database
      .as_ref()
      .begin()
      .await
      .map_err(|e| -> EtwinError { Box::new(e) })?;

    Ok(Self {
      clock,
      database,
      uuid_generator,
    })
  }
}

#[async_trait]
impl<TyClock, TyDatabase, TyUuidGenerator> JobStore for PgJobStore<TyClock, TyDatabase, TyUuidGenerator>
where
  TyClock: Clock,
  TyDatabase: ApiRef<PgPool>,
  TyUuidGenerator: UuidGenerator,
{
  async fn create_job(&self, task_state: &StoredTaskState) -> Result<ShortStoredTask, EtwinError> {
    let start_time = self.clock.now();
    let job_id = JobId::from_uuid(self.uuid_generator.next());

    let mut tx = self.database.as_ref().begin().await?;

    sqlx::query(
      r"
        INSERT INTO etwin_jobs(
          etwin_job_id, ctime
        ) VALUES (
          $1::ETWIN_JOB_ID, $2::INSTANT
        );
      ",
    )
    .bind(&job_id)
    .bind(&start_time)
    .execute(&mut tx)
    .await?;

    let task = insert_task_raw(
      &mut tx,
      start_time,
      TaskId::from_uuid(self.uuid_generator.next()),
      job_id,
      None,
      task_state,
    )
    .await?;

    tx.commit().await?;

    Ok(task)
  }

  async fn create_subtask(&self, task_state: &StoredTaskState, parent: TaskId) -> Result<ShortStoredTask, EtwinError> {
    #[derive(sqlx::FromRow)]
    struct JobIdRow {
      etwin_job_id: JobId,
    }

    let start_time = self.clock.now();
    let mut tx = self.database.as_ref().begin().await?;

    let job_id = sqlx::query_as::<_, JobIdRow>(
      r"
        SELECT etwin_job_id FROM etwin_tasks
        WHERE etwin_task_id = $1::ETWIN_TASK_ID;
      ",
    )
    .bind(&parent)
    .fetch_one(&mut tx)
    .await?
    .etwin_job_id;

    let task = insert_task_raw(
      &mut tx,
      start_time,
      TaskId::from_uuid(self.uuid_generator.next()),
      job_id,
      Some(parent),
      task_state,
    )
    .await?;

    sqlx::query(
      r"
        UPDATE etwin_tasks
        SET _blocking_children = _blocking_children + 1
        WHERE etwin_task_id = $1::ETWIN_TASK_ID;
      ",
    )
    .bind(&parent)
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(task)
  }

  async fn update_task(&self, options: &UpdateTaskOptions<'_>) -> Result<ShortStoredTask, UpdateTaskError> {
    fn map_error<E: Error + Send + Sync + 'static>(err: E) -> UpdateTaskError {
      UpdateTaskError::Other(err.into())
    }

    #[derive(Debug, sqlx::FromRow)]
    struct Row {
      old_status: TaskStatus,
      old_steps: i64,
      ctime: Instant,
      parent_task_id: Option<TaskId>,
      etwin_job_id: JobId,
      step_count: i64,
      running_time: PgInterval,
    }

    let step_interval = duration_to_pg_interval(options.step_time).map_err(map_error)?;
    let now = self.clock.now();
    let mut tx = self.database.as_ref().begin().await.map_err(map_error)?;

    let row = sqlx::query_as::<_, Row>(
      r"
          UPDATE etwin_tasks tasks SET
            step_count = tasks.step_count + 1,
            atime = $2::INSTANT,
            running_time = tasks.running_time + $3::INTERVAL,
            status = $4::ETWIN_TASK_STATUS,
            status_message = $5::TEXT,
            state = $6::JSON
          FROM (
            SELECT * from etwin_tasks
            WHERE etwin_task_id = $1::ETWIN_TASK_ID
            FOR UPDATE
          ) old
          WHERE tasks.etwin_task_id = old.etwin_task_id
          RETURNING
            old.status AS old_status, old.step_count AS old_steps, tasks.etwin_job_id,
            tasks.ctime, tasks.parent_task_id, tasks.step_count, tasks.running_time;
        ",
    )
    .bind(&options.id)
    .bind(&now)
    .bind(&step_interval)
    .bind(&options.status)
    .bind(&options.status_message)
    .bind(&options.state.get())
    .fetch_optional(&mut tx)
    .await
    .map_err(map_error)?;

    let row = match row {
      Some(row) => {
        if row.old_steps != i64::from(options.current_step) {
          return Err(UpdateTaskError::StepConflict {
            task: options.id,
            expected: u32::try_from(row.old_steps).map_err(map_error)?,
            actual: options.current_step,
          });
        }
        if !row.old_status.can_transition_to(options.status) {
          return Err(UpdateTaskError::InvalidTransition {
            task: options.id,
            old: row.old_status,
            new: options.status,
          });
        }
        row
      }
      None => return Err(UpdateTaskError::NotFound(options.id)),
    };

    if let Some(parent) = row.parent_task_id.filter(|_| options.status == TaskStatus::Complete) {
      sqlx::query(
        r"
          UPDATE etwin_tasks
          SET _blocking_children = _blocking_children - 1
          WHERE etwin_task_id = $1::ETWIN_TASK_ID;
        ",
      )
      .bind(&parent)
      .execute(&mut tx)
      .await
      .map_err(map_error)?;
    }

    tx.commit().await.map_err(map_error)?;

    Ok(ShortStoredTask {
      id: options.id,
      job_id: row.etwin_job_id,
      parent: row.parent_task_id,
      status: options.status,
      status_message: options.status_message.map(|m| m.to_string()),
      created_at: row.ctime,
      advanced_at: now,
      step_count: u32::try_from(row.step_count).map_err(map_error)?,
      running_time: pg_interval_to_duration(row.running_time).map_err(map_error)?,
    })
  }

  async fn update_job_status(&self, job: JobId, status: TaskStatus) -> Result<(), EtwinError> {
    // Because only `Running` tasks can transition to a new state,
    // we can find updatable tasks with `WHERE status = 'Running'`.
    // TODO: put the transition rules in the pgsql schema?

    if status == TaskStatus::Running {
      return Ok(());
    }

    let mut tx = self.database.as_ref().begin().await?;
    let now = self.clock.now();

    if status == TaskStatus::Complete {
      // Update _blocking_children for each updated task
      sqlx::query(
        r"
          WITH running_children AS (
            SELECT COUNT(*) AS children, parent_task_id AS task_id
            FROM etwin_tasks
            WHERE status = 'Running' AND etwin_job_id = $1::ETWIN_JOB_ID
            GROUP BY parent_task_id
          )
          UPDATE etwin_tasks SET
            _blocking_children = _blocking_children - running_children.children
          FROM running_children
          WHERE etwin_tasks.etwin_task_id = running_children.task_id;
        ",
      )
      .bind(&job)
      .execute(&mut tx)
      .await?;
    }

    // Update task statuses.
    sqlx::query(
      r"
        UPDATE etwin_tasks SET
          atime = $1::INSTANT,
          status = $2::ETWIN_TASK_STATUS,
          step_count = step_count + 1,
        WHERE status = 'Running' AND etwin_job_id = $3::ETWIN_JOB_ID;
      ",
    )
    .bind(&now)
    .bind(&status)
    .bind(&job)
    .execute(&mut tx)
    .await?;

    tx.commit().await?;
    Ok(())
  }

  async fn get_task(&self, task: TaskId) -> Result<Option<StoredTask>, EtwinError> {
    let row = sqlx::query_as::<_, StoredTaskRow>(
      r"
        SELECT
          etwin_task_id, ctime, atime, etwin_job_id, parent_task_id, status, status_message,
          step_count, running_time, kind, data_version, options::TEXT, state::TEXT
        FROM etwin_tasks WHERE etwin_task_id = $1::ETWIN_TASK_ID;
      ",
    )
    .bind(&task)
    .fetch_optional(self.database.as_ref())
    .await?;

    row.map(TryFrom::try_from).transpose()
  }

  async fn get_job(&self, job: JobId) -> Result<Option<StoredJob>, EtwinError> {
    #[derive(sqlx::FromRow)]
    struct Row {
      etwin_job_id: JobId,
      ctime: Instant,
      etwin_task_id: TaskId,
    }

    let row = sqlx::query_as::<_, Row>(
      r"
        SELECT
          jobs.etwin_job_id, jobs.ctime, tasks.etwin_task_id
        FROM etwin_jobs jobs
        LEFT JOIN etwin_tasks tasks
        ON (tasks.etwin_job_id = jobs.etwin_job_id AND tasks.parent_task_id IS NULL)
        WHERE jobs.etwin_job_id = $1::ETWIN_JOB_ID;
      ",
    )
    .bind(&job)
    .fetch_optional(self.database.as_ref())
    .await?;

    Ok(row.map(|job| StoredJob {
      id: job.etwin_job_id,
      created_at: job.ctime,
      root_task: job.etwin_task_id,
    }))
  }

  async fn get_next_task_to_run(&self) -> Result<Option<StoredTask>, EtwinError> {
    let row = sqlx::query_as::<_, StoredTaskRow>(
      r#"
        SELECT
          etwin_task_id, ctime, atime, etwin_job_id, parent_task_id, status, status_message,
          step_count, running_time, kind, data_version, options::TEXT, state::TEXT
        FROM etwin_tasks WHERE status = 'Running' AND _blocking_children = 0
        ORDER BY atime ASC LIMIT 1;
        "#,
    )
    .fetch_optional(self.database.as_ref())
    .await?;

    row.map(TryFrom::try_from).transpose()
  }
}

#[cfg(test)]
mod test {
  use super::PgJobStore;
  use crate::test::TestApi;
  use chrono::{TimeZone, Utc};
  use etwin_core::clock::VirtualClock;
  use etwin_core::job::JobStore;
  use etwin_core::uuid::Uuid4Generator;
  use etwin_db_schema::force_create_latest;
  use serial_test::serial;
  use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
  use sqlx::PgPool;
  use std::sync::Arc;

  async fn make_test_api() -> TestApi<Arc<VirtualClock>, Arc<dyn JobStore>> {
    let config = etwin_config::find_config(std::env::current_dir().unwrap()).unwrap();
    let database: PgPool = PgPoolOptions::new()
      .max_connections(5)
      .connect_with(
        PgConnectOptions::new()
          .host(&config.db.host)
          .port(config.db.port)
          .database(&config.db.name)
          .username(&config.db.user)
          .password(&config.db.password),
      )
      .await
      .unwrap();
    force_create_latest(&database, true).await.unwrap();

    let database = Arc::new(database);

    let clock = Arc::new(VirtualClock::new(Utc.ymd(2020, 1, 1).and_hms(0, 0, 0)));
    let uuid_generator = Arc::new(Uuid4Generator);
    let job_store: Arc<dyn JobStore> = Arc::new(
      PgJobStore::new(Arc::clone(&clock), Arc::clone(&database), uuid_generator)
        .await
        .unwrap(),
    );

    TestApi { clock, job_store }
  }

  test_job_store!(
    #[serial]
    || make_test_api().await
  );
}

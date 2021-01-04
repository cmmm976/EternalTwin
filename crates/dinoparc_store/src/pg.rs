use async_trait::async_trait;
use etwin_core::api::ApiRef;
use etwin_core::clock::Clock;
use etwin_core::core::Instant;
use etwin_core::dinoparc::{
  ArchivedDinoparcUser, DinoparcServer, DinoparcStore, DinoparcUserId, DinoparcUsername, GetDinoparcUserOptions,
  ShortDinoparcUser,
};
use sqlx;
use sqlx::PgPool;
use std::error::Error;

pub struct PgDinoparcStore<TyClock, TyDatabase>
where
  TyClock: Clock,
  TyDatabase: ApiRef<PgPool>,
{
  clock: TyClock,
  database: TyDatabase,
}

impl<TyClock, TyDatabase> PgDinoparcStore<TyClock, TyDatabase>
where
  TyClock: Clock,
  TyDatabase: ApiRef<PgPool>,
{
  pub fn new(clock: TyClock, database: TyDatabase) -> Self {
    Self { clock, database }
  }
}

#[async_trait]
impl<TyClock, TyDatabase> DinoparcStore for PgDinoparcStore<TyClock, TyDatabase>
where
  TyClock: Clock,
  TyDatabase: ApiRef<PgPool>,
{
  async fn get_short_user(
    &self,
    options: &GetDinoparcUserOptions,
  ) -> Result<Option<ArchivedDinoparcUser>, Box<dyn Error>> {
    #[derive(Debug, sqlx::FromRow)]
    struct Row {
      dinoparc_server: DinoparcServer,
      dinoparc_user_id: DinoparcUserId,
      username: DinoparcUsername,
      archived_at: Instant,
    }

    let row: Option<Row> = sqlx::query_as::<_, Row>(
      r"
      SELECT dinoparc_server, dinoparc_user_id, username, archived_at
      FROM dinoparc_users
      WHERE dinoparc_server = $1::DINOPARC_SERVER AND dinoparc_user_id = $2::DINOPARC_USER_ID;
    ",
    )
    .bind(&options.server)
    .bind(&options.id)
    .fetch_optional(self.database.as_ref())
    .await?;

    Ok(row.map(|r| ArchivedDinoparcUser {
      server: r.dinoparc_server,
      id: r.dinoparc_user_id,
      username: r.username,
      archived_at: r.archived_at,
    }))
  }

  async fn touch_short_user(&self, short: &ShortDinoparcUser) -> Result<ArchivedDinoparcUser, Box<dyn Error>> {
    let now = self.clock.now();
    sqlx::query(
      r"
      INSERT INTO dinoparc_users(dinoparc_server, dinoparc_user_id, username, archived_at)
      VALUES ($1::DINOPARC_SERVER, $2::DINOPARC_USER_ID, $3::DINOPARC_USERNAME, $4::INSTANT)
        ON CONFLICT (dinoparc_server, dinoparc_user_id)
          DO UPDATE SET username = $3::DINOPARC_USERNAME;
    ",
    )
    .bind(&short.server)
    .bind(&short.id)
    .bind(&short.username)
    .bind(now)
    .fetch_optional(self.database.as_ref())
    .await?;
    Ok(ArchivedDinoparcUser {
      server: short.server,
      id: short.id.clone(),
      username: short.username.clone(),
      archived_at: now,
    })
  }
}

#[cfg(feature = "neon")]
impl<TyClock, TyDatabase> neon::prelude::Finalize for PgDinoparcStore<TyClock, TyDatabase>
where
  TyClock: Clock,
  TyDatabase: ApiRef<PgPool>,
{
}

#[cfg(test)]
mod test {
  use super::PgDinoparcStore;
  use crate::test::TestApi;
  use chrono::{TimeZone, Utc};
  use etwin_core::clock::VirtualClock;
  use etwin_core::dinoparc::DinoparcStore;
  use etwin_db_schema::force_create_latest;
  use serial_test::serial;
  use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
  use sqlx::PgPool;
  use std::sync::Arc;

  async fn make_test_api() -> TestApi<Arc<VirtualClock>, Arc<dyn DinoparcStore>> {
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
    force_create_latest(&database).await.unwrap();

    let database = Arc::new(database);

    let clock = Arc::new(VirtualClock::new(Utc.timestamp(1607531946, 0)));
    let dinoparc_store: Arc<dyn DinoparcStore> =
      Arc::new(PgDinoparcStore::new(Arc::clone(&clock), Arc::clone(&database)));

    TestApi {
      _clock: clock,
      dinoparc_store,
    }
  }

  #[tokio::test]
  #[serial]
  async fn test_empty() {
    crate::test::test_empty(make_test_api().await).await;
  }

  // #[tokio::test]
  // #[serial]
  // async fn test_touch_user() {
  //   crate::test::test_touch_user(make_test_api().await).await;
  // }
}

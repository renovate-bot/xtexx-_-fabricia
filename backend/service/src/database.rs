use std::fmt::Debug;

use deadpool::managed::{Manager, Object, Pool, PoolError, RecycleError, RecycleResult};
use diesel::{Connection, ConnectionError, SqliteConnection};
use diesel_async::{AsyncConnection, AsyncPgConnection};
use fabricia_backend_model::db::{BoxedSqlConn, run_migrations};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::Duration;
use tokio::task::spawn_blocking;
use tracing::{info, info_span, warn};

use crate::{Result, redis::RedisService};

/// Configuration for [`DatabaseService`].
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DatabaseConfig {
	/// URL to the primary database server.
	///
	/// For example:
	/// - `postgres://user:password@host/database`
	/// - `sqlite://:memory:`
	/// - `sqlite://data.db`
	pub url: String,
	/// The maximum number of connections managed by the pool.
	///
	/// When using `sqlite://:memory:`, this must be set to 1.
	#[serde(default = "default_max_conns")]
	pub max_connections: usize,
}

fn default_max_conns() -> usize {
	3
}

/// Database connection service.
pub struct DatabaseService {
	pool: Pool<SqlConnectionManager>,
}

impl DatabaseService {
	pub async fn new(config: &DatabaseConfig, redis: &RedisService) -> Result<Self> {
		let manager = SqlConnectionManager(config.to_owned());
		let pool = Pool::builder(manager)
			.max_size(config.max_connections)
			.build()
			.map_err(DatabaseError::from)?;

		{
			let _lock = redis.lock("sql-migration", Duration::minutes(5)).await?;

			let _span = info_span!("running pending migrations").entered();
			info!("running database migrations");
			let conn = pool.manager().create().await?;
			let versions = spawn_blocking(move || run_migrations(conn))
				.await
				.map_err(DatabaseError::from)?
				.map_err(DatabaseError::MigrationError)?;
			for version in versions {
				warn!(%version, "database migration applied");
			}
			info!("database migrations completed");
		}

		let db = Self { pool };

		// for tests, the above migrations are not enough
		// because in memory SQLite database get cleared
		// after re-establishing the connection
		#[cfg(test)]
		{
			let mut conn = db.get().await?;
			fabricia_backend_model::db::run_migrations_sqlite(&mut conn)
				.map_err(DatabaseError::MigrationError)?;
		}

		Ok(db)
	}

	pub async fn get(&self) -> Result<SqlConnRef> {
		Ok(self.pool.get().await.map_err(DatabaseError::from)?)
	}
}

impl Debug for DatabaseService {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DatabaseService")
			.field("config", &self.pool.manager().0)
			.finish()
	}
}

#[derive(Debug)]
pub struct SqlConnectionManager(DatabaseConfig);

pub type SqlConnRef = Object<SqlConnectionManager>;

impl Manager for SqlConnectionManager {
	type Type = BoxedSqlConn;
	type Error = DatabaseError;

	fn create(
		&self,
	) -> impl Future<Output = std::result::Result<BoxedSqlConn, DatabaseError>> + Send {
		async {
			let url = &self.0.url;
			if url.starts_with("postgresql://") || url.starts_with("postgres://") {
				AsyncPgConnection::establish(&url)
					.await
					.map(BoxedSqlConn::Pg)
					.map_err(DatabaseError::ConnectionError)
			} else if let Some(path) = url.strip_prefix("sqlite://") {
				SqliteConnection::establish(path)
					.map(BoxedSqlConn::Sqlite)
					.map_err(DatabaseError::ConnectionError)
			} else {
				Err(DatabaseError::UnknownUrlSchema(url.clone()))
			}
		}
	}

	fn recycle(
		&self,
		obj: &mut BoxedSqlConn,
		_metrics: &deadpool::managed::Metrics,
	) -> impl Future<Output = RecycleResult<DatabaseError>> + Send {
		async {
			if std::thread::panicking() || obj.is_broken() {
				return Err(RecycleError::Message("Broken connection".into()));
			}
			obj.ping().await.map_err(DatabaseError::QueryError)?;
			Ok(())
		}
	}
}

#[derive(Debug, Error)]
pub enum DatabaseError {
	#[error("connection error: {0}")]
	ConnectionError(#[from] ConnectionError),
	#[error("query error: {0}")]
	QueryError(#[from] diesel::result::Error),
	#[error("connection pool error: {0:?}")]
	PoolError(PoolError<()>),
	#[error("connection pool build error: {0}")]
	PoolBuildError(#[from] deadpool::managed::BuildError),
	#[error("async-await joining error: {0}")]
	JoinError(#[from] tokio::task::JoinError),
	#[error("failed to apply migration: {0}")]
	MigrationError(Box<dyn std::error::Error + Send + Sync>),

	#[error("unknown connection URL schema: {0}")]
	UnknownUrlSchema(String),
}

impl From<PoolError<DatabaseError>> for DatabaseError {
	fn from(value: PoolError<DatabaseError>) -> Self {
		Self::PoolError(match value {
			PoolError::Timeout(timeout_type) => PoolError::Timeout(timeout_type),
			PoolError::Backend(err) => return err,
			PoolError::Closed => PoolError::Closed,
			PoolError::NoRuntimeSpecified => PoolError::NoRuntimeSpecified,
			PoolError::PostCreateHook(_) => unreachable!(),
		})
	}
}

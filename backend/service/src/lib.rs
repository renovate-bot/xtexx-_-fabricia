//! Fabricia backend services.

use std::sync::Arc;

use branch::{BranchError, BranchService};
use config::BackendConfig;
use database::{DatabaseError, DatabaseService};
use job_queue::{JobQueue, JobQueueError};
use redis::{RedisError, RedisService};
use target::TargetService;
use thiserror::Error;

pub mod branch;
pub mod bus;
pub mod config;
pub mod database;
pub mod job_queue;
pub mod redis;
pub mod target;

/// Service container for Fabricia backends.
///
/// All services are wrapped with [`Arc`].
#[derive(Debug, Clone)]
pub struct BackendServices {
	pub config: Arc<BackendConfig>,
	pub target: Arc<TargetService>,
	pub database: Arc<DatabaseService>,
	pub redis: Arc<RedisService>,
	pub job_queue: Arc<JobQueue>,
	pub branch: Arc<BranchService>,
}

impl BackendServices {
	#[tracing::instrument(skip(config))]
	pub async fn new(config: BackendConfig) -> Result<Self> {
		let config = Arc::new(config);
		let target = Arc::new(TargetService::new(&config.target)?);
		let database = Arc::new(DatabaseService::new(&config.database).await?);
		let redis = Arc::new(RedisService::new(&config.redis).await?);
		let job_queue = Arc::new(JobQueue::new(database.clone()));
		let branch =
			Arc::new(BranchService::new(database.clone(), job_queue.clone()));

		Ok(Self {
			config,
			target,
			database,
			redis,
			job_queue,
			branch,
		})
	}
}

/// Backend errors.
#[derive(Debug, Error)]
pub enum Error {
	#[error("JSON error: {0}")]
	JsonError(#[from] serde_json::Error),
	#[error(transparent)]
	DatabaseError(#[from] DatabaseError),
	#[error(transparent)]
	RedisError(#[from] RedisError),
	#[error(transparent)]
	JobQueueError(#[from] JobQueueError),
	#[error(transparent)]
	BranchError(#[from] BranchError),
}

/// A specialized [`Result`] for backend errors.
pub type Result<T> = std::result::Result<T, Error>;

impl From<diesel::result::Error> for Error {
	fn from(value: diesel::result::Error) -> Self {
		Self::DatabaseError(DatabaseError::QueryError(value))
	}
}

#[cfg(test)]
pub(crate) mod test {
	use database::DatabaseConfig;
	use crate::redis::RedisConfig;
	use target::*;

	use crate::*;

	pub async fn test_env() -> BackendServices {
		let config = BackendConfig {
			database: DatabaseConfig {
				url: "sqlite://:memory:".to_string(),
				max_connections: 1,
			},
			redis: RedisConfig {
				url: "redis://127.0.0.1".to_string(),
				max_connections: 1,
			},
			target: vec![
				TargetConfig {
					name: "arch1".into(),
					arch: None,
				},
				TargetConfig {
					name: "arch2".into(),
					arch: Some("testarch2".into()),
				},
			],
		};
		BackendServices::new(config).await.unwrap()
	}

	#[tokio::test]
	async fn test_init_services() {
		let env = test_env().await;
		assert!(env.job_queue.fetch_and_start().await.unwrap().is_none());
	}
}

// Redis connection manager.

use std::{fmt::Debug, ops::Deref};

use deadpool::managed::{Manager, Object, Pool, PoolError, RecycleError, RecycleResult};
use rand::Rng;
use redis::{Client, Pipeline, aio::MultiplexedConnection};
use rslock::{Lock, LockManager};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::Duration;

use crate::branch::BranchRef;

/// Configuration for [`RedisService`].
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RedisConfig {
	/// URL to the Redis server.
	///
	/// For example: `redis://127.0.0.1/`.
	pub url: String,
	/// The maximum number of connections managed by the pool.
	#[serde(default = "default_max_conns")]
	pub max_connections: usize,
}

fn default_max_conns() -> usize {
	3
}

impl RedisConfig {
	pub async fn make_client(&self) -> Result<Client, redis::RedisError> {
		Ok(Client::open(self.url.as_str())?)
	}
}

pub struct RedisService {
	pool: Pool<RedisManager>,
	locker: LockManager,
}

impl RedisService {
	pub async fn new(config: &RedisConfig) -> RedisResult<Self> {
		let manager = RedisManager(config.to_owned());
		let pool = Pool::builder(manager)
			.max_size(config.max_connections)
			.build()?;

		let locker = LockManager::new(vec![config.url.clone()]);

		Ok(Self { pool, locker })
	}

	pub async fn get(&self) -> RedisResult<RedisConnRef> {
		Ok(self.pool.get().await?)
	}

	pub async fn make_client(&self) -> RedisResult<Client> {
		Ok(self.pool.manager().0.make_client().await?)
	}

	pub async fn lock<K: Into<LockKey>>(&self, key: K, ttl: Duration) -> RedisResult<LockGuard> {
		let key = key.into().to_key();
		let mut delay = Duration::milliseconds(50);
		loop {
			match self.locker.lock(key.as_bytes(), ttl.try_into()?).await {
				Ok(lock) => return Ok(lock.into()),
				Err(rslock::LockError::TtlTooLarge) => {
					return Err(rslock::LockError::TtlTooLarge.into());
				}
				Err(_) => {
					tokio::time::sleep(delay.try_into()?).await;
					if delay <= Duration::seconds(3) {
						delay *= 2;
					}
					continue;
				}
			}
		}
	}
}

impl Debug for RedisService {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RedisService").finish()
	}
}

#[derive(Debug, Error)]
pub enum RedisError {
	#[error(transparent)]
	RedisError(#[from] redis::RedisError),
	#[error("connection pool error: {0:?}")]
	PoolError(#[from] PoolError<redis::RedisError>),
	#[error("connection pool build error: {0}")]
	PoolBuildError(#[from] deadpool::managed::BuildError),
	#[error("distributed lock error: {0}")]
	LockError(#[from] rslock::LockError),
	#[error("time conversion error: {0}")]
	TimeConversionError(#[from] time::error::ConversionRange),
}

pub type RedisResult<T> = Result<T, RedisError>;

pub struct RedisManager(RedisConfig);

pub type RedisConnRef = Object<RedisManager>;

impl Manager for RedisManager {
	type Type = MultiplexedConnection;
	type Error = redis::RedisError;

	async fn create(&self) -> Result<Self::Type, Self::Error> {
		Ok(self
			.0
			.make_client()
			.await?
			.get_multiplexed_tokio_connection()
			.await?)
	}

	async fn recycle(
		&self,
		obj: &mut Self::Type,
		_metrics: &deadpool::managed::Metrics,
	) -> RecycleResult<Self::Error> {
		let ping = rand::rng().random::<u64>().to_string();
		let (n,) = Pipeline::with_capacity(2)
			.cmd("UNWATCH")
			.ignore()
			.cmd("PING")
			.arg(&ping)
			.query_async::<(String,)>(obj)
			.await?;
		if n == ping {
			Ok(())
		} else {
			Err(RecycleError::message("Invalid PING response"))
		}
	}
}

/// Key for distributed locking
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum LockKey {
	Branch(BranchRef),
	Misc(&'static str),
}

impl LockKey {
	pub fn to_key(&self) -> String {
		match self {
			LockKey::Branch(branch) => format!("lock:branch:{}", branch),
			LockKey::Misc(key) => format!("lock:misc:{}", key),
		}
	}
}

impl From<&'static str> for LockKey {
	fn from(value: &'static str) -> Self {
		Self::Misc(value)
	}
}

#[derive(Debug)]
pub struct LockGuard(rslock::Lock);

impl From<Lock> for LockGuard {
	fn from(lock: Lock) -> Self {
		Self(lock)
	}
}

impl LockGuard {
	pub async fn extend(&mut self, ttl: Duration) -> RedisResult<()> {
		self.0 = self.0.lock_manager.extend(&self.0, ttl.try_into()?).await?;
		Ok(())
	}
}

impl Drop for LockGuard {
	fn drop(&mut self) {
		// force clone the lock
		let lock = Lock {
			resource: self.0.resource.to_owned(),
			val: self.0.val.to_owned(),
			validity_time: self.0.validity_time,
			lock_manager: self.0.lock_manager.to_owned(),
		};
		tokio::task::spawn(async move {
			lock.lock_manager.unlock(&lock).await;
		});
	}
}

impl AsRef<Lock> for LockGuard {
	fn as_ref(&self) -> &Lock {
		&self.0
	}
}

impl Deref for LockGuard {
	type Target = Lock;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

use serde::{Deserialize, Serialize};

use crate::{database::DatabaseConfig, redis::RedisConfig, target::TargetConfig};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
pub struct BackendConfig {
	pub database: DatabaseConfig,
	pub redis: RedisConfig,
	pub target: Vec<TargetConfig>,
}

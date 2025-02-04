use fabricia_backend_service::{
	config::BackendConfig, database::DatabaseConfig, redis::RedisConfig,
	target::TargetConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
pub struct CrayonConfig {
	pub web: WebConfig,
	pub database: DatabaseConfig,
	pub redis: RedisConfig,
	pub target: Vec<TargetConfig>,
}

impl TryFrom<CrayonConfig> for BackendConfig {
	type Error = anyhow::Error;

	fn try_from(config: CrayonConfig) -> Result<Self, Self::Error> {
		Ok(BackendConfig {
			database: config.database,
			redis: config.redis,
			target: config.target,
		})
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
pub struct WebConfig {
	/// Address for the web server to listen on.
	///
	/// Examples:
	/// - `unix://crayon.socket`
	/// - `tcp://127.0.0.1:8000`
	pub listen: String,
}

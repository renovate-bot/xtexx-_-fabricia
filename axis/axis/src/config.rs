use fabricia_backend_service::{
	config::BackendConfig, database::DatabaseConfig, redis::RedisConfig, target::TargetConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
pub struct AxisConfig {
	pub http: HttpConfig,
	pub database: DatabaseConfig,
	pub redis: RedisConfig,
	pub target: Vec<TargetConfig>,
	pub runners: usize,
}

impl TryFrom<AxisConfig> for BackendConfig {
	type Error = anyhow::Error;

	fn try_from(config: AxisConfig) -> Result<Self, Self::Error> {
		Ok(BackendConfig {
			database: config.database,
			redis: config.redis,
			target: config.target,
		})
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
pub struct HttpConfig {
	/// Address for the web server to listen on.
	///
	/// Examples:
	/// - `unix://crayon.socket`
	/// - `tcp://127.0.0.1:8000`
	pub listen: String,
}

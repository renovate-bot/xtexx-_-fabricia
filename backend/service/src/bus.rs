//! Backend bus

use std::{fmt::Debug, sync::Arc};

use fabricia_backend_model::bus::{BackendBusMessage, C2ABusMessage};
use futures::future::BoxFuture;

use crate::{Result, redis::RedisService};

pub trait BackendBusService
where
	Self: Send + Sync + Debug,
{
	fn broadcast(&self, message: BackendBusMessage) -> BoxFuture<'_, Result<()>>;
	fn send_c2a(&self, message: C2ABusMessage) -> BoxFuture<'_, Result<()>>;
}

pub type BoxedBusService = Box<dyn BackendBusService + 'static>;

pub trait BackendBusFactory {
	fn construct(self, redis: Arc<RedisService>) -> BoxFuture<'static, Result<BoxedBusService>>;
}

pub const BACKEND_BUS_CHANNEL: &str = "bus:backend";
pub const BACKEND_BUS_C2A_CHANNEL: &str = "bus:c2a";

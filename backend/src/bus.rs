//! Backend bus

use std::{fmt::Debug, sync::Arc};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::{Result, redis::RedisService};

/// A backend bus message that can be broadcasted across the backend bus.
///
/// Backend bus messages will be received by all Axis and Crayon
/// instances listening on the bus.
///
/// This kind of message can be used to flush in memory caches across the backend.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum BackendBusMessage {}

/// A backend bus message from Crayon to Axis.
///
/// Not all Axis instances will receive the posted C2A message.
/// When attempting to post a C2A bus message from a Axis instance,
/// the message will be immediately handled locally, and will not be
/// published to other instances.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum C2ABusMessage {
	ResumeJobRunner,
}

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

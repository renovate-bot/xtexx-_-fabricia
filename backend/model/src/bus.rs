//! Backend bus

use serde::{Deserialize, Serialize};

use crate::branch::BranchRef;

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

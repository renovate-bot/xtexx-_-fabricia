//! Backend bus

use serde::{Deserialize, Serialize};

use crate::branch::BranchRef;

/// A backend bus message from Crayon to Axis.
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

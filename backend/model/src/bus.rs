//! Backend bus

use kstring::KString;
use serde::{Deserialize, Serialize};

/// A backend bus message from Crayon to Axis.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum C2ABusMessage {
	ResumeJobRunner,
}

/// Key for distributed locking
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum LockKey {
	Branch(KString),
}

impl LockKey {
	pub fn to_key(&self) -> String {
		match self {
			LockKey::Branch(branch) => format!("lock:branch:{}", branch),
		}
	}
}

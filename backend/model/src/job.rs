use kstring::KString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::branch::BranchRef;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "c", rename = "kebab-case")]
pub enum JobCommand {
	/// Synchronize metadata of a branch.
	SyncBranch(BranchRef),
}

impl JobCommand {
	pub fn serialize(
		&self,
	) -> serde_json::Result<(KString, serde_json::Value)> {
		let mut value = serde_json::to_value(self)?;
		Ok((
			KString::from_ref(value["t"].as_str().unwrap()),
			value.as_object_mut().unwrap().remove("c").unwrap(),
		))
	}

	pub fn deserialize(
		kind: &str,
		value: serde_json::Value,
	) -> serde_json::Result<Self> {
		let value = serde_json::json!({ "t": kind, "c": value });
		serde_json::from_value(value)
	}
}

pub type JobRef = Uuid;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Job {
	pub id: JobRef,
	pub command: JobCommand,
}

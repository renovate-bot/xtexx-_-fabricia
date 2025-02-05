use fabricia_common_model::branch::{BranchStatus, TrackingMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ApiBranchInfo {
	pub name: String,
	pub base: Option<String>,
	pub status: BranchStatus,
	pub priority: u16,
	pub tracking_mode: TrackingMode,
	pub commit: Option<String>,
	pub packages: u32,
}

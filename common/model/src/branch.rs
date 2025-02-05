use serde::{Deserialize, Serialize};

/// State of a branch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BranchStatus {
	/// State for branches needing refresh.
	///
	/// In this state, all pending build jobs will be paused and wait for
	/// the branch to be refreshed (ready).
	Dirty,
	/// State for branches ready to start packaging.
	///
	/// Only in this state, pending build jobs may be dispatched.
	Ready,
	/// State for branches with branch-level errors.
	///
	/// No pending build jobs can be dispatched in this state.
	/// This usually requires manual restart by users.
	Error { reason: String },
	/// State for branches paused by maintainers.
	///
	/// Only on a maintainer's command, a branch enters this state.
	/// And, only on maintainer's command, a suspended branch resumes into dirty state.
	Suspended { reason: String },
}

/// Tracking rules indicating how should we track packages in a branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
	/// Tracking all changed (in comparison with base branch) packages automatically.
	Auto,
	/// Do not track any packages.
	Unmanaged,
}

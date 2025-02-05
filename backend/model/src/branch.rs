use fabricia_common_model::branch::{BranchStatus, TrackingMode};

pub type BranchRef = i64;

/// State of a branch.
///
/// Stored as a tiny unsigned column. Unknown values are decoded as suspended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SqlBranchStatus {
	/// State for branches needing refresh.
	///
	/// On entering this state, the caller should enqueue
	/// a branch synchronization job to transform the branch
	/// into next state.
	///
	/// In this state, all pending build jobs will be paused and wait for
	/// the branch to be ready.
	Dirty = 0,
	/// State for branches ready to start packaging.
	///
	/// On entering this state, the caller ensures all build graphs of
	/// this branch is up-to-date, and no branch-level errors are found.
	///
	/// Only in this state, pending build jobs may be dispatched.
	Ready = 1,
	/// State for branches with branch-level errors.
	///
	/// When a branch-level error is raised, a branch transforms into this state.
	/// No pending build jobs can be dispatched in this state.
	///
	/// This usually requires restart.
	Error = 2,
	/// State for branches paused by maintainers.
	///
	/// Only on a maintainer's command, a branch enters this state.
	/// And, only on maintainer's command, a suspended branch resumes into dirty state.
	Suspended = 3,
}

impl From<u8> for SqlBranchStatus {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for SqlBranchStatus {
	fn from(value: i16) -> Self {
		match value {
			0 => Self::Dirty,
			1 => Self::Ready,
			2 => Self::Error,
			3 => Self::Suspended,
			_ => Self::Suspended,
		}
	}
}

impl SqlBranchStatus {
	pub fn into_common(&self, message: Option<String>) -> BranchStatus {
		match self {
			SqlBranchStatus::Dirty => BranchStatus::Dirty,
			SqlBranchStatus::Ready => BranchStatus::Ready,
			SqlBranchStatus::Error => BranchStatus::Error {
				reason: message.unwrap_or_default(),
			},
			SqlBranchStatus::Suspended => BranchStatus::Suspended {
				reason: message.unwrap_or_default(),
			},
		}
	}
}

/// Database representation of [TrackingMode].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SqlTrackingMode {
	/// [TrackingMode::Auto]
	Auto = 0,
	/// [TrackingMode::Unmanaged]
	Unmanaged = 1,
}

impl From<u8> for SqlTrackingMode {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for SqlTrackingMode {
	fn from(value: i16) -> Self {
		match value {
			0 => Self::Auto,
			1 => Self::Unmanaged,
			_ => Self::Unmanaged,
		}
	}
}

impl From<TrackingMode> for SqlTrackingMode {
	fn from(value: TrackingMode) -> Self {
		match value {
			TrackingMode::Auto => Self::Auto,
			TrackingMode::Unmanaged => Self::Unmanaged,
		}
	}
}

impl From<SqlTrackingMode> for TrackingMode {
	fn from(value: SqlTrackingMode) -> Self {
		match value {
			SqlTrackingMode::Auto => Self::Auto,
			SqlTrackingMode::Unmanaged => Self::Unmanaged,
		}
	}
}

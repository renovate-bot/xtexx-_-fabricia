/// State of a branch.
///
/// Stored as a tiny unsigned column. Unknown values are decoded as suspended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BranchState {
	/// State for branches needing refresh.
	///
	/// On entering this state, the caller should enqueue
	/// a branch synchronization job to transform the branch
	/// into next state.
	///
	/// In this state, all pending build jobs will be paused and wait for
	/// the branch to be ready.
	#[default]
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

impl From<u8> for BranchState {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for BranchState {
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

/// State of a package.
///
/// Stored as a tiny unsigned column. Unknown values are decoded as error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PackageState {
	/// State for packages needing a metadata refresh.
	///
	/// On entering this state, the caller should enqueue
	/// a package evaluation job to transform the branch
	/// into next state.
	///
	/// In this state, all pending build jobs will be paused and wait for
	/// the metadata to be ready.
	#[default]
	Dirty = 0,
	/// State for packages ready to start packaging.
	///
	/// On entering this state, the caller ensures all build graphs of
	/// this package is up-to-date, and no source-package-level errors are found.
	///
	/// Only in this state, pending build jobs may be dispatched.
	Ready = 1,
	/// State for package with source-package-level errors.
	///
	/// No pending build jobs can be dispatched in this state.
	Error = 2,
}

impl From<u8> for PackageState {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for PackageState {
	fn from(value: i16) -> Self {
		match value {
			0 => Self::Dirty,
			1 => Self::Ready,
			2 => Self::Error,
			_ => Self::Error,
		}
	}
}

/// State of a (package, target).
///
/// Stored as a tiny unsigned column. Unknown values are decoded as error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PackageTargetState {
	#[default]
	Dirty = 0,
	Ready = 1,
	BuildFailed = 2,
	Error = 3,
}

impl From<u8> for PackageTargetState {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for PackageTargetState {
	fn from(value: i16) -> Self {
		match value {
			0 => Self::Dirty,
			1 => Self::Ready,
			2 => Self::BuildFailed,
			3 => Self::Error,
			_ => Self::Error,
		}
	}
}

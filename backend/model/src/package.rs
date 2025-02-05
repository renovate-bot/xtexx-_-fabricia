/// State of a package.
///
/// Stored as a tiny unsigned column. Unknown values are decoded as error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SqlPackageStatus {
	/// State for packages needing a metadata refresh.
	///
	/// On entering this state, the caller should enqueue
	/// a package evaluation job to transform the branch
	/// into next state.
	///
	/// In this state, all pending build jobs will be paused and wait for
	/// the metadata to be ready.
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

impl From<u8> for SqlPackageStatus {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for SqlPackageStatus {
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
pub enum SqlPackageTargetState {
	#[default]
	Dirty = 0,
	Ready = 1,
	BuildFailed = 2,
	Error = 3,
}

impl From<u8> for SqlPackageTargetState {
	fn from(value: u8) -> Self {
		Self::from(value as i16)
	}
}

impl From<i16> for SqlPackageTargetState {
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

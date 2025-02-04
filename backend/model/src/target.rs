use std::hash::{DefaultHasher, Hash, Hasher};

use kstring::KString;

/// Hashed target identifier
///
/// This is calculated by [TargetInfo::make_id].
pub type TargetId = u64;

/// Information related to a build target.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TargetInfo {
	pub id: TargetId,
	/// Name of the target.
	pub name: KString,
	/// AOSC OS architecture name
	pub arch: KString,
}

impl TargetInfo {
	pub fn make_id<S: AsRef<str>>(name: S) -> TargetId {
		let mut hasher = DefaultHasher::new();
		name.as_ref().hash(&mut hasher);
		hasher.finish()
	}
}

impl PartialOrd for TargetInfo {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id.partial_cmp(&other.id)
	}
}

impl Ord for TargetInfo {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id.cmp(&other.id)
	}
}

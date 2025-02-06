use std::{
	collections::HashMap,
	hash::{DefaultHasher, Hash, Hasher},
	sync::Arc,
};

use kstring::KString;
use serde::{Deserialize, Serialize};

use crate::Result;

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

#[derive(Debug)]
pub struct TargetService {
	by_id: HashMap<TargetId, Arc<TargetInfo>>,
	by_name: HashMap<KString, Arc<TargetInfo>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct TargetConfig {
	pub name: KString,
	pub arch: Option<KString>,
}

impl TargetService {
	pub fn new(config: &Vec<TargetConfig>) -> Result<Self> {
		let mut service = Self {
			by_id: HashMap::new(),
			by_name: HashMap::new(),
		};

		for target in config {
			let id = TargetInfo::make_id(&target.name);
			let arch = target.arch.clone().unwrap_or_else(|| target.name.clone());

			let target = Arc::new(TargetInfo {
				id,
				name: target.name.clone(),
				arch,
			});
			service.by_id.insert(id, target.clone());
			service.by_name.insert(target.name.clone(), target);
		}

		Ok(service)
	}
}

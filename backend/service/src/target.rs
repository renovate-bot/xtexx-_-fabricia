use std::{collections::HashMap, sync::Arc};

use fabricia_backend_model::target::{TargetId, TargetInfo};
use kstring::KString;
use serde::{Deserialize, Serialize};

use crate::Result;

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

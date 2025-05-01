//! Git local tree service.

use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use git2::{ErrorCode, Repository};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Git tree service for job runners.
///
/// To give job runners access to the build script tree,
/// a bare clone of the build script repository.
///
/// In some cases, runners may want to create new references to
/// commits, etc. For example, it is important to tag old branch
/// references so when the branch is updated in the future, Fabricia
/// can compare changes to the build scripts easily.
///
/// Thus, a new working remote is introduced. After updating working
/// references, they will be pushed to the working remote to share
/// them across the backend network. For a single-instance deployment,
/// the working remote can be omitted and no synchronization will be
/// performed.
pub struct GitService {
	config: GitTreeConfig,
}

impl Debug for GitService {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GitService")
			.field("config", &self.config)
			.finish()
	}
}

/// Configuration for [`GitService`].
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GitTreeConfig {
	#[serde(default = "default_local_path")]
	pub local_path: PathBuf,
	#[serde(default = "default_origin")]
	pub origin: String,
	#[serde(default)]
	pub working_remote: Option<String>,
}

fn default_local_path() -> PathBuf {
	PathBuf::from("tree")
}

fn default_origin() -> String {
	"https://github.com/AOSC-Dev/aosc-os-abbs".to_string()
}

pub const REMOTE_ORIGIN: &str = "origin";
pub const REMOTE_WORKING: &str = "working";

impl GitService {
	pub fn new(config: GitTreeConfig) -> Result<Self> {
		let repo = if !config.local_path.join("config").exists() {
			info!("initializing local git working repository");
			Repository::init_bare(&config.local_path)?
		} else {
			Repository::open(&config.local_path)?
		};
		ensure_git_remote(&repo, REMOTE_ORIGIN, &config.origin)?;
		if let Some(remote) = &config.working_remote {
			ensure_git_remote(&repo, REMOTE_WORKING, remote)?;
		}

		Ok(Self { config })
	}

	/// Opens the working repository.
	pub fn open(&self) -> Result<Repository> {
		Ok(Repository::open(&self.config.local_path)?)
	}
}

fn ensure_git_remote(repo: &Repository, name: &str, url: &str) -> Result<()> {
	match repo.find_remote(REMOTE_ORIGIN) {
		Ok(remote) => {
			if remote.url() != Some(url) {
				info!(remote = name, url, "updating remote URL of local git repo");
				repo.remote_set_url(name, url)?;
			}
			if remote.pushurl_bytes().is_some() {
				info!(remote = name, "clearing remote push URL of local git repo");
				repo.remote_set_pushurl(name, None)?;
			}
		}
		Err(error) => {
			if error.code() == ErrorCode::NotFound {
				info!(remote = name, "adding new remote to local git repo");
				repo.remote(name, url)?;
			} else {
				return Err(error.into());
			}
		}
	}
	Ok(())
}

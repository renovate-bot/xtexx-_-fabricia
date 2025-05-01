use anyhow::{Result, anyhow};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, delete};
use fabricia_backend::{
	BackendServices,
	branch::{BranchRef, SqlBranchStatus, SqlTrackingMode},
	db::schema::{self, branch::dsl},
};
use fabricia_common::branch::TrackingMode;
use git2::{ObjectType, Oid, Repository, Tree};
use kstring::KString;
use tokio::task::block_in_place;
use tracing::{info, warn};

use crate::git::{GitService, REMOTE_ORIGIN};

pub async fn sync_branch(
	backend: &BackendServices,
	git: &GitService,
	branch: BranchRef,
) -> Result<()> {
	let mut conn = backend.database.get().await?;

	#[derive(Debug, Queryable, Selectable)]
	#[diesel(table_name = schema::branch)]
	struct SqlBranchInfo {
		name: String,
		base: Option<i64>,
		status: i16,
		tracking: i16,
		commit: Option<Vec<u8>>,
	}

	let info: SqlBranchInfo = conn
		.load_one_select(dsl::branch.filter(dsl::id.eq(branch)))
		.await?;
	assert_ne!(info.base, Some(branch));
	let base_info: Option<SqlBranchInfo> = match info.base {
		Some(base) => Some(
			conn.load_one_select(dsl::branch.filter(dsl::id.eq(base)))
				.await?,
		),
		None => None,
	};
	let repo = git.open()?;
	let status = SqlBranchStatus::from(info.status);
	if status == SqlBranchStatus::Suspended {
		warn!(branch, "cannot sync suspended branch");
	}
	let tracking = TrackingMode::from(SqlTrackingMode::from(info.tracking));

	info!(branch, "fetching branch from origin repository");
	block_in_place(|| {
		repo.find_remote(REMOTE_ORIGIN)?.fetch(
			&[format!("refs/heads/{}", info.name)],
			None,
			Some("Branch synchronization (target)"),
		)
	})?;
	if let Some(base) = base_info.as_ref().map(|info| &info.name) {
		block_in_place(|| {
			repo.find_remote(REMOTE_ORIGIN)?.fetch(
				&[format!("refs/heads/{}", base)],
				None,
				Some("Branch synchronization (base)"),
			)
		})?;
	}

	let new_commit = repo
		.find_reference(&format!("refs/remotes/{}/{}", REMOTE_ORIGIN, info.name))?
		.peel_to_commit()?;
	let base_commit = base_info
		.as_ref()
		.map(|base| {
			repo.find_reference(&format!("refs/remotes/{}/{}", REMOTE_ORIGIN, base.name))?
				.peel_to_commit()
		})
		.transpose()?;
	let last_commit = info
		.commit
		.map(|commit| Oid::from_bytes(&commit))
		.transpose()?
		.map(|commit| repo.find_commit(commit))
		.transpose()?;
	if let Some(last_commit) = &last_commit {
		if last_commit.id() == new_commit.id() {
			info!(branch, "branch reference has not been changed");
			return Ok(());
		}
		info!(branch, commit = %new_commit.id(), old_commit = %last_commit.id(), "syncing branch");
	} else {
		info!(branch, commit = %new_commit.id(), "doing initial sync of branch");
	}

	info!(branch, "collecting changed packages");
	let diff_base = last_commit
		.or(base_commit)
		.map(|commit| commit.tree())
		.transpose()?;
	let new_tree = new_commit.tree()?;
	let diff_packages = diff_packages_by_tree(&repo, diff_base.as_ref(), &new_tree)?;
	info!(
		branch,
		diff = diff_packages.len(),
		"finished diffing packages"
	);

	Ok(())
}

pub async fn untrack_branch(backend: &BackendServices, branch: BranchRef) -> Result<()> {
	let mut conn = backend.database.get().await?;
	conn.transaction::<(), anyhow::Error, _>(async |conn| {
		conn.execute(delete(dsl::branch).filter(dsl::id.eq(branch)))
			.await?;
		conn.execute(
			delete(schema::pkg_target::table).filter(schema::pkg_target::branch.eq(branch)),
		)
		.await?;
		conn.execute(delete(schema::pkg::table).filter(schema::pkg::branch.eq(branch)))
			.await?;
		Ok(())
	})
	.await?;
	info!(branch, "untracked branch");
	Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum TreePackageDiffKind {
	/// A new package has been added or branched.
	New,
	/// A package has been updated.
	Updated,
	/// A package has been reset to the base state.
	Reseted,
}

/// Collects changed packages by comparing two trees.
///
/// This only shows new or changed packages. Dropped packages
/// are not listed because they are not needed for packaging.
///
/// Returns a iterator of pairs of package section and package name.
/// All tree objects must be accessible locally.
///
/// If the base tree is `None`, all packages will be returned.
fn diff_packages_by_tree(
	repo: &Repository,
	base: Option<&Tree>,
	tree: &Tree,
) -> Result<Vec<(KString, KString)>> {
	let mut results = Vec::new();
	for new_sec in tree {
		if new_sec.kind() != Some(ObjectType::Tree) {
			continue;
		}
		let sec = new_sec
			.name()
			.ok_or_else(|| anyhow!("section name in new tree is not valid UTF-8"))?;
		if !sec.contains('-') {
			continue;
		}
		let new_sec = new_sec.to_object(repo)?.peel_to_tree()?;
		let base_sec = base
			.and_then(|tree| tree.get_name(sec))
			.map(|tree| tree.to_object(repo))
			.transpose()?
			.take_if(|tree| tree.kind() == Some(ObjectType::Tree))
			.map(|tree| tree.peel_to_tree())
			.transpose()?;

		for new_pkg in &new_sec {
			let pkg = new_pkg
				.name()
				.ok_or_else(|| anyhow!("package name in new tree is not valid UTF-8"))?;
			let base_pkg = base_sec
				.as_ref()
				.and_then(|sec| sec.get_name(pkg))
				.map(|pkg| pkg.id());
			if base_pkg == Some(new_pkg.id()) {
				continue;
			}
			results.push((KString::from_ref(sec), KString::from_ref(pkg)));
		}
	}
	Ok(results)
}

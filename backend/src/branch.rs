use std::sync::Arc;

use diesel::{
	ExpressionMethods, OptionalExtension, QueryDsl, delete, insert_into,
	prelude::{AsChangeset, Identifiable},
	update,
};
use fabricia_common_model::branch::{BranchStatus, TrackingMode};
use kstring::KString;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

use crate::{
	Result,
	db::{
		schema::{self, branch::dsl},
		service::DatabaseService,
	},
	job_queue::{JobCommand, JobQueue},
};

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

#[derive(Debug)]
pub struct BranchService {
	db: Arc<DatabaseService>,
	job_queue: Arc<JobQueue>,
}

impl BranchService {
	pub fn new(db: Arc<DatabaseService>, job_queue: Arc<JobQueue>) -> Self {
		Self { db, job_queue }
	}

	/// Tracks a new branch.
	pub async fn track(&self, name: &str, info: BranchConfigInfo) -> Result<()> {
		let mut conn = self.db.get().await?;
		let branch = name.to_owned();

		conn.transaction::<(), crate::BackendError, _>(async |conn| {
			let base = match info.base {
				Some(base) => Some(self.find_id_or_err(base).await?),
				None => None,
			};
			let priority = info.priority.unwrap_or(100) as u16;

			let id = conn
				.get_result::<_, i64>(
					insert_into(dsl::branch)
						.values((
							dsl::name.eq(&branch),
							dsl::status.eq(SqlBranchStatus::Dirty as i16),
							dsl::base.eq(base),
							dsl::priority.eq(priority as i16),
							dsl::tracking.eq(SqlTrackingMode::from(
								info.tracking_mode.unwrap_or(TrackingMode::Auto),
							) as i16),
						))
						.returning(dsl::id),
				)
				.await?;
			self.job_queue
				.enqueue_with_priority(conn, JobCommand::SyncBranch(id), priority)
				.await?;

			Ok(())
		})
		.await?;
		info!(branch, "tracked branch");

		Ok(())
	}

	pub async fn find_id<S: AsRef<str>>(&self, name: S) -> Result<Option<BranchRef>> {
		let mut conn = self.db.get().await?;
		Ok(conn
			.get_result(
				dsl::branch
					.filter(dsl::name.eq(name.as_ref()))
					.select(dsl::id),
			)
			.await
			.optional()?)
	}

	pub async fn find_id_or_err<S: AsRef<str>>(&self, name: S) -> Result<BranchRef> {
		Ok(self
			.find_id(&name)
			.await?
			.ok_or_else(|| BranchError::BranchNameNotFound(KString::from_ref(name.as_ref())))?)
	}

	/// Untracks a new branch.
	pub async fn untrack(&self, id: BranchRef) -> Result<()> {
		let mut conn = self.db.get().await?;

		conn.transaction::<(), crate::BackendError, _>(async |conn| {
			non_zero_or_not_found(
				conn.execute(delete(dsl::branch).filter(dsl::id.eq(id)))
					.await?,
				id,
			)?;

			Ok(())
		})
		.await?;
		info!(id, "untracked branch");

		Ok(())
	}

	pub async fn update_config(&self, id: BranchRef, info: &BranchConfigInfo) -> Result<()> {
		let mut conn = self.db.get().await?;
		let base = match &info.base {
			Some(base) => {
				if base.is_empty() {
					Some(None)
				} else {
					Some(Some(self.find_id_or_err(base).await?))
				}
			}
			None => None,
		};

		non_zero_or_not_found(
			conn.execute(update(dsl::branch).set(SqlBranchConfig {
				id,
				base,
				priority: info.priority.map(|pri| pri as i16),
				tracking: info.tracking_mode.map(|mode| mode as i16),
			}))
			.await?,
			id,
		)?;
		Ok(())
	}
}

#[derive(Debug, Error)]
pub enum BranchError {
	#[error("branch {0} not found")]
	BranchNameNotFound(KString),
	#[error("branch {0} not found")]
	BranchNotFound(BranchRef),
}

fn non_zero_or_not_found(val: usize, id: BranchRef) -> Result<(), BranchError> {
	if val == 0 {
		Err(BranchError::BranchNotFound(id))
	} else {
		Ok(())
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
pub struct BranchConfigInfo {
	/// Name of the base branch of this branch.
	///
	/// Set this to empty string to remove base branch.
	pub base: Option<KString>,
	pub priority: Option<u16>,
	pub tracking_mode: Option<TrackingMode>,
}

#[derive(Debug, Identifiable, AsChangeset)]
#[diesel(table_name = schema::branch)]
pub struct SqlBranchConfig {
	id: BranchRef,
	base: Option<Option<BranchRef>>,
	priority: Option<i16>,
	tracking: Option<i16>,
}

#[cfg(test)]
mod test {
	use diesel::QueryDsl;

	use crate::{db::schema::branch::dsl, job_queue::JobCommand, test::test_env};

	#[tokio::test]
	async fn test_track() {
		let env = test_env().await;
		env.branch.track("test", Default::default()).await.unwrap();

		// assert object
		let mut db = env.database.get().await.unwrap();
		assert_eq!(
			db.get_result::<_, (String, i16)>(dsl::branch.select((dsl::name, dsl::status)))
				.await
				.unwrap(),
			("test".to_string(), 0)
		);
		drop(db);

		// assert sync job
		let job = env.job_queue.fetch_and_start().await.unwrap().unwrap();
		assert_eq!(job.command, JobCommand::SyncBranch(1));
	}
}

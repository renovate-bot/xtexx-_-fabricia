use std::sync::Arc;

use diesel::{ExpressionMethods, insert_into};
use fabricia_backend_model::{
	db::{schema::branch::dsl, types::BranchState},
	job::JobCommand,
};
use thiserror::Error;
use tracing::info;

use crate::{Result, database::DatabaseService, job_queue::JobQueue};

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
	pub async fn track(&self, name: &str) -> Result<()> {
		let mut conn = self.db.get().await?;
		let branch = name.to_owned();

		conn.transaction::<(), crate::Error, _>(async |conn| {
			let id = conn
				.get_result::<_, i64>(
					insert_into(dsl::branch)
						.values((
							dsl::name.eq(&branch),
							dsl::state.eq(BranchState::Dirty as i16),
						))
						.returning(dsl::id),
				)
				.await? as u64;
			self.job_queue
				.enqueue(conn, JobCommand::SyncBranch(id))
				.await?;

			Ok(())
		})
		.await?;
		info!(branch, "tracked branch");

		Ok(())
	}
}

#[derive(Debug, Error)]
pub enum BranchError {}

#[cfg(test)]
mod test {
	use diesel::QueryDsl;
	use fabricia_backend_model::{db::schema::branch::dsl, job::JobCommand};

	use crate::test::test_env;

	#[tokio::test]
	async fn test_track() {
		let env = test_env().await;
		env.branch.track("test").await.unwrap();

		// assert object
		let mut db = env.database.get().await.unwrap();
		assert_eq!(
			db.get_result::<_, (String, i16)>(
				dsl::branch.select((dsl::name, dsl::state))
			)
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

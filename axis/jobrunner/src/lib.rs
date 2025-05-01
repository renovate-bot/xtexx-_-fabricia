use std::sync::Arc;

use anyhow::Result;
use fabricia_backend::{BackendServices, job_queue::JobCommand};
use git::GitService;
use tokio::sync::Notify;
use tracing::{Instrument, debug, error, info, info_span};

mod branch;
pub mod git;

#[derive(Debug)]
pub struct JobRunner {
	/// Notifier to resume the dispatcher immediately.
	notifier: Notify,
	/// Backend services
	backend: Arc<BackendServices>,
	git: Arc<GitService>,
}

impl JobRunner {
	pub fn new(backend: Arc<BackendServices>, git: Arc<GitService>) -> Result<Self> {
		Ok(Self {
			notifier: Notify::const_new(),
			backend,
			git,
		})
	}

	#[tracing::instrument(level = "info", name = "jobrunner", skip(self))]
	pub async fn run(self: Arc<Self>, index: usize) {
		info!("job runner started");
		loop {
			self.notifier.notified().await;
			debug!("notified to resume");

			let result = async {
				while let Some(job) = self.backend.job_queue.fetch_and_start().await? {
					info!(job = %job.id, "starting job");
					let mut db = self.backend.database.get().await?;
					self.exec(job.command)
						.instrument(info_span!("execute job", job = %job.id))
						.await?;
					self.backend.job_queue.finish_job(&mut db, job.id).await?;
					info!(job = %job.id, "finished job");
				}
				Ok::<_, anyhow::Error>(())
			}
			.await;
			if let Err(error) = result {
				error!(?error, "job runner error")
			}
		}
	}

	#[tracing::instrument(level = "debug", name = "job_watcher", skip(self))]
	pub async fn run_watcher(self: Arc<Self>, runners: usize) {
		info!("job watcher started");
		loop {
			let result = async {
				let count = self.backend.job_queue.count_pending(runners).await?;
				for _ in 0..count {
					self.notify_one();
				}

				Ok::<_, anyhow::Error>(())
			}
			.await;
			if let Err(error) = result {
				error!(?error, "job watcher error")
			}
			tokio::time::sleep(std::time::Duration::from_secs(3 * 60)).await;
		}
	}

	pub fn notify_one(&self) {
		self.notifier.notify_one();
	}

	pub fn notify_all(&self) {
		self.notifier.notify_waiters();
	}

	/// Runs a job command.
	async fn exec(&self, job: JobCommand) -> Result<()> {
		match job {
			JobCommand::SyncBranch(branch) => {
				branch::sync_branch(&self.backend, &self.git, branch).await
			}
			JobCommand::UntrackBranch(branch) => {
				branch::untrack_branch(&self.backend, branch).await
			}
		}
	}
}

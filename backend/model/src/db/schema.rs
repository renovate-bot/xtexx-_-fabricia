diesel::table! {
	branch (id) {
		id -> BigInt,
		/// Name of the branch.
		///
		/// This should be equal to the Git branch name, and should not
		/// be changed after branch insertion.
		name -> Varchar,
		state -> Int2,
		status -> Varchar,
		/// Priority of this branch.
		///
		/// By default, the base priority should be 100.
		priority -> Int2,
		/// Count of tracked packages in this branch.
		total_srcpkgs->Int4,
	}
}

diesel::table! {
	use crate::db::utils::*;
	use diesel::sql_types::*;

	job_queue (id) {
		/// Unique identifier of this job.
		///
		/// The ID must be a UUID v7, of which timestamp is the time when
		/// the job is enqueued.
		id -> XUuid,
		kind -> VarChar,
		data -> XJson,
		priority -> Int2,
		/// Started time of this job.
		///
		/// This column is null when and only when the job is not started.
		started_at -> Nullable<Timestamp>
	}
}

diesel::table! {
	use crate::db::utils::*;
	use diesel::sql_types::*;

	/// Table for tracked packages.
	pkg (id) {
		id -> XUuid,
		branch -> BigInt,
		name -> VarChar,
		section -> VarChar,
		state -> Int2,
		status -> VarChar,
		data -> XJson,
	}
}

diesel::table! {
	use crate::db::utils::*;
	use diesel::sql_types::*;

	/// Table for (tracked packages, target).
	///
	/// Unsupported pairs (i.e. target is included in `FAIL_ARCH`)
	/// should not be in this table.
	pkg_target (id) {
		id -> XUuid,
		branch -> BigInt,
		package -> XUuid,
		target -> BigInt,
		state -> Int2,
		data -> XJson,
	}
}

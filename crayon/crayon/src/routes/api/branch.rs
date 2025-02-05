use std::collections::HashMap;

use axum::{
	Json,
	extract::{Path, State},
	http::StatusCode,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, Queryable, Selectable};
use fabricia_backend_model::{
	branch::*,
	db::{
		schema::{self, branch::dsl},
		utils::WherePredicate,
	},
};
use fabricia_backend_service::{BackendServices, branch::BranchConfigInfo, database::SqlConnRef};
use fabricia_common_model::branch::TrackingMode;
use fabricia_crayon_api_model::branch::*;
use serde::{Deserialize, Serialize};

use super::{
	auth::AuthRequired,
	error::{ApiError, ApiResult, OptionExt},
};

pub async fn list_branches(
	State(backend): State<BackendServices>,
) -> ApiResult<Json<HashMap<String, ApiBranchInfo>>> {
	let mut db = backend.database.get().await?;
	let result: Vec<SqlApiBranchInfo> = db.load_select(dsl::branch).await?;
	let mut output = HashMap::with_capacity(result.len());
	for info in result {
		output.insert(info.name.clone(), info.into_api(&mut db).await?);
	}

	Ok(Json(output))
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = schema::branch)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqlApiBranchInfo {
	name: String,
	base: Option<i64>,
	status: i16,
	status_msg: Option<String>,
	priority: i16,
	tracking: i16,
	commit: Option<Vec<u8>>,
	total_srcpkgs: i32,
}

impl SqlApiBranchInfo {
	async fn into_api(self, db: &mut SqlConnRef) -> ApiResult<ApiBranchInfo> {
		let base = match self.base {
			None => None,
			Some(base) => db
				.get_result(
					dsl::branch
						.select(dsl::name)
						.filter(dsl::id.eq(base))
						.limit(1),
				)
				.await
				.optional()?,
		};
		let status = SqlBranchStatus::from(self.status).into_common(self.status_msg);
		let tracking_mode = TrackingMode::from(SqlTrackingMode::from(self.tracking));
		let commit = self.commit.map(hex::encode);
		Ok(ApiBranchInfo {
			name: self.name.clone(),
			base,
			status,
			priority: self.priority as u16,
			tracking_mode,
			commit,
			packages: self.total_srcpkgs as u32,
		})
	}
}

pub async fn get_branch(
	State(backend): State<BackendServices>,
	Path(name): Path<String>,
) -> ApiResult<Json<ApiBranchInfo>> {
	let mut db = backend.database.get().await?;
	get_branch_info(&mut db, dsl::name.eq(name)).await
}

async fn get_branch_info<F: WherePredicate<dsl::branch>>(
	db: &mut SqlConnRef,
	filter: F,
) -> ApiResult<Json<ApiBranchInfo>> {
	let result: SqlApiBranchInfo = db
		.load_one_select(dsl::branch.limit(1).filter(filter))
		.await?;
	Ok(Json(result.into_api(db).await?))
}

pub async fn new_branch(
	AuthRequired: AuthRequired,
	State(backend): State<BackendServices>,
	Path(name): Path<String>,
	Json(info): Json<BranchConfigInfo>,
) -> ApiResult<(StatusCode, Json<ApiBranchInfo>)> {
	if backend.branch.find_id(&name).await?.is_some() {
		return Err(ApiError::CustomRef(
			StatusCode::NOT_ACCEPTABLE,
			"branch has already been tracked",
		));
	}

	backend.branch.track(&name, info).await?;

	let mut db = backend.database.get().await?;
	Ok((
		StatusCode::CREATED,
		get_branch_info(&mut db, dsl::name.eq(name)).await?,
	))
}

pub async fn update_branch_config(
	AuthRequired: AuthRequired,
	State(backend): State<BackendServices>,
	Path(name): Path<String>,
	Json(info): Json<BranchConfigInfo>,
) -> ApiResult<(StatusCode, Json<ApiBranchInfo>)> {
	let id = backend
		.branch
		.find_id(&name)
		.await?
		.or_api_error(StatusCode::NOT_FOUND, "branch not found")?;
	backend.branch.update_config(id, &info).await?;

	let mut db = backend.database.get().await?;
	Ok((
		StatusCode::ACCEPTED,
		get_branch_info(&mut db, dsl::name.eq(name)).await?,
	))
}

pub async fn delete_branch(
	AuthRequired: AuthRequired,
	State(backend): State<BackendServices>,
	Path(name): Path<String>,
) -> ApiResult<(StatusCode, &'static str)> {
	let id = backend
		.branch
		.find_id(name)
		.await?
		.or_api_error(StatusCode::NOT_FOUND, "branch not found")?;
	backend.branch.untrack(id).await?;
	Ok((StatusCode::ACCEPTED, "branch deleted"))
}

use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::get};
use fabricia_backend_service::BackendServices;

use crate::config::CrayonConfig;

mod api;

pub fn make_router(config: Arc<CrayonConfig>, backend_services: BackendServices) -> Result<Router> {
	let router = Router::new()
		.route("/", get(handler))
		.nest("/api/v0", api::api_router())
		.with_state(backend_services)
		.with_state(config);

	Ok(router)
}

async fn handler() -> &'static str {
	concat!("Fabricia Crayon ", env!("CARGO_PKG_VERSION"))
}

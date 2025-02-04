use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::get};
use fabricia_backend_service::BackendServices;

use crate::config::CrayonConfig;

pub fn make_router(
	config: Arc<CrayonConfig>,
	backend_services: BackendServices,
) -> Result<Router> {
	let router = Router::new()
		.route("/", get(handler))
		.with_state(config)
		.with_state(backend_services);

	Ok(router)
}

async fn handler() -> &'static str {
	concat!("Fabricia Crayon ", env!("CARGO_PKG_VERSION"))
}

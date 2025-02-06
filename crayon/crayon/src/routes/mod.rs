use anyhow::Result;
use axum::{Router, routing::get};

use crate::CrayonServices;

mod api;

pub fn make_router(services: CrayonServices) -> Result<Router> {
	let router = Router::new()
		.route("/", get(handler))
		.nest("/api/v0", api::api_router())
		.with_state(services);

	Ok(router)
}

async fn handler() -> &'static str {
	concat!("Fabricia Crayon ", env!("CARGO_PKG_VERSION"))
}

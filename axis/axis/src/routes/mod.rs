use anyhow::Result;
use axum::{Router, routing::get};

use crate::AxisServices;

pub fn make_router(services: AxisServices) -> Result<Router> {
	let router = Router::new().route("/", get(handler)).with_state(services);

	Ok(router)
}

async fn handler() -> &'static str {
	concat!("Fabricia Axis ", env!("CARGO_PKG_VERSION"))
}

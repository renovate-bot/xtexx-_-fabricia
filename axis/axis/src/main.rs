use std::{
	fs,
	path::PathBuf,
	sync::{Arc, OnceLock},
};

use anyhow::{Result, bail};
use bus::AxisBusFactory;
use clap::Parser;
use config::AxisConfig;
use fabricia_axis_jobrunner::JobRunner;
use fabricia_backend_service::BackendServices;
use tokio::net::{TcpListener, UnixListener};
use tracing::info;

mod bus;
mod config;
mod routes;

#[derive(clap::Parser)]
struct Args {
	#[arg(short, long, default_value = "axis.toml")]
	config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();

	tracing::subscriber::set_global_default(
		tracing_subscriber::FmtSubscriber::builder()
			.with_max_level(tracing::Level::INFO)
			.finish(),
	)?;

	let config_path = &args.config;
	let config = toml::from_str::<AxisConfig>(&fs::read_to_string(config_path)?)?;
	info!("loaded configuration from file: {:?}", config_path);

	info!("initializing backend services ...");
	let services_ref = Arc::new(OnceLock::new());
	let backend_services = Arc::new(
		BackendServices::new(
			config.clone().try_into()?,
			AxisBusFactory(services_ref.clone()),
		)
		.await?,
	);
	info!("initializing runner service ...");
	let runner = JobRunner::new(backend_services.clone())?;
	let services = AxisServices {
		config: Arc::new(config),
		backend: backend_services,
		runner: Arc::new(runner),
	};
	services_ref.set(services.clone()).unwrap();

	tokio::spawn(bus::handle_bus_message(services.clone()));
	for i in 0..=services.config.runners {
		tokio::spawn(services.runner.clone().run(i));
	}
	tokio::spawn(services.runner.clone().run_watcher(services.config.runners));

	let listen_addr = services.config.http.listen.clone();
	let router = routes::make_router(services)?;
	if let Some(path) = listen_addr.strip_prefix("unix://") {
		let path = PathBuf::from(path);
		_ = fs::remove_file(&path);
		fs::create_dir_all(path.parent().unwrap())?;

		let listener = UnixListener::bind(&path)?;
		info!("listening on UDS: {:?}", path);
		axum::serve(listener, router).await?;
	} else if let Some(addr) = listen_addr.strip_prefix("tcp://") {
		let listener = TcpListener::bind(addr).await?;
		info!("listening on TCP {}", listener.local_addr()?);
		axum::serve(listener, router).await.unwrap();
	} else {
		bail!("unsupported http.listen schema")
	}

	Ok(())
}

#[derive(Debug, Clone)]
pub struct AxisServices {
	pub config: Arc<AxisConfig>,
	pub backend: Arc<BackendServices>,
	pub runner: Arc<JobRunner>,
}

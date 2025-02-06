use std::{fs, path::PathBuf, sync::Arc};

use anyhow::{Result, bail};
use bus::CrayonBusFactory;
use clap::Parser;
use config::CrayonConfig;
use fabricia_backend::BackendServices;
use tokio::net::{TcpListener, UnixListener};
use tracing::info;

mod bus;
mod config;
mod routes;

#[derive(clap::Parser)]
struct Args {
	#[arg(short, long, default_value = "crayon.toml")]
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
	let config = toml::from_str::<CrayonConfig>(&fs::read_to_string(config_path)?)?;
	info!("loaded configuration from file: {:?}", config_path);

	info!("initializing backend services ...");
	let backend_services =
		BackendServices::new(config.clone().try_into()?, CrayonBusFactory).await?;
	info!("initialized backend services");
	let services = CrayonServices {
		config,
		backend: Arc::new(backend_services),
	};

	tokio::spawn(bus::handle_bus_message(services.clone()));

	let listen_addr = services.config.web.listen.clone();
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
		bail!("unsupported web.listen schema")
	}

	Ok(())
}

#[derive(Debug, Clone)]
pub struct CrayonServices {
	pub config: CrayonConfig,
	pub backend: Arc<BackendServices>,
}

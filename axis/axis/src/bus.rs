//! [BackendBusService] implementation for Axis.

use std::sync::{Arc, OnceLock};

use fabricia_backend_model::bus::{BackendBusMessage, C2ABusMessage};
use fabricia_backend_service::{
	Result,
	bus::{
		BACKEND_BUS_C2A_CHANNEL, BACKEND_BUS_CHANNEL, BackendBusFactory, BackendBusService,
		BoxedBusService,
	},
	redis::{RedisError, RedisService},
};
use futures::{
	FutureExt, StreamExt,
	future::{BoxFuture, ready},
};
use redis::AsyncCommands;
use tracing::{debug, error, info};

use crate::AxisServices;

#[derive(Debug)]
pub struct AxisBusService {
	redis: Arc<RedisService>,
	services: Arc<OnceLock<AxisServices>>,
}

impl BackendBusService for AxisBusService {
	fn broadcast(&self, message: BackendBusMessage) -> BoxFuture<'_, Result<()>> {
		async move {
			let message = serde_json::to_string(&message)?;
			let _: () = self
				.redis
				.get()
				.await?
				.publish(BACKEND_BUS_CHANNEL, message.as_str())
				.await
				.map_err(RedisError::RedisError)?;
			Ok(())
		}
		.boxed()
	}

	fn send_c2a(&self, message: C2ABusMessage) -> BoxFuture<'_, Result<()>> {
		async move {
			if let Some(services) = self.services.get() {
				if let Err(error) = process_c2a_message(message, services).await {
					error!(%error, "failed to handle local-looped C2A bus message");
				}
			}
			Ok(())
		}
		.boxed()
	}
}

pub struct AxisBusFactory(pub Arc<OnceLock<AxisServices>>);

impl BackendBusFactory for AxisBusFactory {
	fn construct(self, redis: Arc<RedisService>) -> BoxFuture<'static, Result<BoxedBusService>> {
		ready(Ok(Box::new(AxisBusService {
			redis,
			services: self.0,
		}) as Box<dyn BackendBusService>))
		.boxed()
	}
}

pub async fn handle_bus_message(services: AxisServices) {
	let client = services.backend.redis.make_client().await.unwrap();
	let mut pubsub = client.get_async_pubsub().await.unwrap();
	pubsub.subscribe(BACKEND_BUS_CHANNEL).await.unwrap();
	pubsub.subscribe(BACKEND_BUS_C2A_CHANNEL).await.unwrap();
	info!("subscribed to backend bus channel");
	while let Some(msg) = pubsub.on_message().next().await {
		let channel = msg.get_channel_name();
		let payload = msg.get_payload::<String>();
		let payload = match payload {
			Ok(value) => value,
			Err(error) => {
				error!(channel, %error, "failed to decode bus message");
				continue;
			}
		};
		match channel {
			BACKEND_BUS_CHANNEL => {
				let result = handle_backend_bus_message(payload, &services).await;
				if let Err(error) = result {
					error!(channel, %error, "failed to handle backend bus message");
				}
			}
			BACKEND_BUS_C2A_CHANNEL => {
				let result = handle_c2a_bus_message(payload, &services).await;
				if let Err(error) = result {
					error!(channel, %error, "failed to handle C2A bus message");
				}
			}
			_ => {
				error!(channel, "received bus message from unknown channel");
			}
		}
	}
}

async fn handle_backend_bus_message(
	message: String,
	_services: &AxisServices,
) -> anyhow::Result<()> {
	let message = serde_json::from_str::<BackendBusMessage>(&message)?;
	debug!(?message, "received backend bus message");
	Ok(())
}

async fn handle_c2a_bus_message(message: String, services: &AxisServices) -> anyhow::Result<()> {
	let message = serde_json::from_str::<C2ABusMessage>(&message)?;
	process_c2a_message(message, services).await
}

async fn process_c2a_message(
	message: C2ABusMessage,
	services: &AxisServices,
) -> anyhow::Result<()> {
	debug!(?message, "processing C2A bus message");
	match message {
		C2ABusMessage::ResumeJobRunner => services.runner.notify_one(),
	}
	Ok(())
}

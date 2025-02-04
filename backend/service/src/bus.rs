//! Backend bus

use fabricia_backend_model::bus::C2ABusMessage;
use futures::future::BoxFuture;

pub trait BackendBusService {
	fn send_c2a(&self, message: C2ABusMessage) -> BoxFuture<'_, ()>;
}

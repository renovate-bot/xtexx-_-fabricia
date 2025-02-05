use axum::{extract::FromRequestParts, http::request::Parts};

use super::error::ApiError;

pub struct AuthRequired;

impl<S> FromRequestParts<S> for AuthRequired
where
	S: Send + Sync,
{
	type Rejection = ApiError;

	async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
		// TODO: implement authorization
		if true {
			Ok(Self)
		} else {
			Err(ApiError::AuthRequired)
		}
	}
}

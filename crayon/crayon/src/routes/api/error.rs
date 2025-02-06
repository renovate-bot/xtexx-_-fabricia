use axum::{
	http::StatusCode,
	response::{AppendHeaders, IntoResponse, Response},
};
use fabricia_backend::BackendError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
	#[error(transparent)]
	BackendError(BackendError),

	#[error("api error: {1}")]
	CustomRef(StatusCode, &'static str),
	#[error("api error: {1}")]
	CustomString(StatusCode, String),

	#[error("authentication is required")]
	AuthRequired,
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		if let ApiError::CustomRef(status, message) = self {
			(status, message).into_response()
		} else if let ApiError::CustomString(status, message) = self {
			(status, message).into_response()
		} else if let ApiError::AuthRequired = self {
			(
				StatusCode::UNAUTHORIZED,
				AppendHeaders([("WWW-Authenticate", "Bearer")]),
				"authentication is required",
			)
				.into_response()
		} else {
			(StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
		}
	}
}

impl<T: Into<BackendError>> From<T> for ApiError {
	fn from(value: T) -> Self {
		Self::BackendError(value.into())
	}
}

pub(crate) type ApiResult<T> = Result<T, ApiError>;

pub(crate) trait IntoCustomApiError {
	fn into_custom_api_error(self, status: StatusCode) -> ApiError;
}

impl IntoCustomApiError for &'static str {
	fn into_custom_api_error(self, status: StatusCode) -> ApiError {
		ApiError::CustomRef(status, self)
	}
}
impl IntoCustomApiError for String {
	fn into_custom_api_error(self, status: StatusCode) -> ApiError {
		ApiError::CustomString(status, self)
	}
}

pub(crate) trait OptionExt<T> {
	fn or_api_error<M: IntoCustomApiError>(
		self,
		status: StatusCode,
		message: M,
	) -> Result<T, ApiError>;
}

impl<T> OptionExt<T> for Option<T> {
	fn or_api_error<M: IntoCustomApiError>(
		self,
		status: StatusCode,
		message: M,
	) -> Result<T, ApiError> {
		match self {
			Some(val) => Ok(val),
			None => Err(message.into_custom_api_error(status)),
		}
	}
}

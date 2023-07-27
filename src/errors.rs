use anyhow::{anyhow, Error};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub enum ServerError {
    DbError(Error),
    ValidationError(Error),
    NoContent(Error),
    GitHubAPIError(Error),
    OpenAIAPIError(Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        match self {
            ServerError::DbError(err) => {
                tracing::error!("{:?}", err);
                HTTPError::iternal_error().into_response()
            }
            ServerError::ValidationError(err) => {
                tracing::error!("{:?}", err);
                HTTPError::new(err)
                    .with_status(StatusCode::BAD_REQUEST)
                    .into_response()
            }
            ServerError::NoContent(err) => {
                tracing::error!("{:?}", err);
                HTTPError::new(err)
                    .with_status(StatusCode::NO_CONTENT)
                    .into_response()
            }
            ServerError::GitHubAPIError(err) | ServerError::OpenAIAPIError(err) => {
                tracing::error!("{:?}", err);
                HTTPError::iternal_error().into_response()
            }
        }
    }
}

#[derive(Debug)]
struct HTTPError {
    error: Error,
    status_code: StatusCode,
}

impl HTTPError {
    fn new(error: Error) -> Self {
        Self {
            error,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    const fn with_status(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    fn iternal_error() -> Self {
        Self {
            error: anyhow!("Internal error"),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for HTTPError {
    fn into_response(self) -> Response {
        (
            self.status_code,
            Json(json!({ "error": self.error.to_string() })),
        )
            .into_response()
    }
}

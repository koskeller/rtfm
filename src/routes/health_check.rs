use axum::Json;
use serde_json::{json, Value};

use crate::errors::ServerError;

pub async fn health_check_handler() -> Result<Json<Value>, ServerError> {
    Ok(Json(json!({ "status": "ok" })))
}

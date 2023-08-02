use axum::http::HeaderName;
use hyper::Request;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};

#[derive(Clone, Default)]
pub struct Id;

impl MakeRequestId for Id {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let id = uuid::Uuid::new_v4().to_string().parse().unwrap();
        Some(RequestId::new(id))
    }
}

/// Adds `X-Request-Id` header to request with randomly generated UUID.
pub fn request_id_layer() -> SetRequestIdLayer<Id> {
    let x_request_id = HeaderName::from_static("x-request-id");
    SetRequestIdLayer::new(x_request_id.clone(), Id::default())
}

/// Propagate `X-Request-Id`s from requests to responses.
pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    let x_request_id = HeaderName::from_static("x-request-id");
    PropagateRequestIdLayer::new(x_request_id)
}

use crate::{
    builder::Builder,
    sse::SseListener,
    types::{Bid, SignedVersionedResponse},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use eth2::types::{ChainSpec, ExecutionBlockHash, MainnetEthSpec, PublicKeyBytes, SecretKey, Slot};
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;

pub use crate::error::Error;

mod builder;
mod error;
mod sse;
mod types;

type E = MainnetEthSpec;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let url = "http://localhost:5052".to_string();
    let cache_size = NonZeroUsize::new(16).unwrap();
    let sse_listener = SseListener::new(url, cache_size);
    let secret_key = SecretKey::random();
    let spec = ChainSpec::mainnet();
    let builder = Arc::new(Builder::new(secret_key, sse_listener.clone(), spec));

    // Spawn event listener on its own thread.
    let _handle = tokio::spawn(async move { sse_listener.listen() });

    let app = Router::new()
        .route("/eth/v1/builder/validators", post(register))
        .route(
            "/eth/v1/builder/header/:slot/:parent_hash/:pubkey",
            get(get_header),
        )
        .route("/eth/v1/builder/blinded_blocks", post(unblind))
        .route("/eth/v1/builder/status", get(status))
        .with_state(builder);

    let addr = SocketAddr::from(([127, 0, 0, 1], 18550));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn register() {
    // Don't care about registrations, return 200 OK.
}

pub async fn get_header(
    State(builder): State<Arc<Builder>>,
    Path(slot): Path<Slot>,
    Path(parent_hash): Path<ExecutionBlockHash>,
    _: Path<PublicKeyBytes>,
) -> Result<Json<SignedVersionedResponse<Bid<E>>>, (StatusCode, String)> {
    match builder.get_header::<E>(slot, parent_hash).await {
        Ok(header) => Ok(Json(header)),
        Err(Error::NoPayload) => Err((StatusCode::NO_CONTENT, "no payload available".into())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}"))),
    }
}

pub async fn status() {
    // Always healthy.
}

pub async fn unblind() -> (StatusCode, String) {
    // TODO.
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "not implemented".to_string(),
    )
}

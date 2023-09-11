use crate::{builder::Builder, config::Config, sse::SseListener};
use axum::{
    extract::{rejection::PathRejection, Path, State, TypedHeader},
    headers::UserAgent,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use eth2::types::builder_bid::SignedBuilderBid;
use eth2::types::{
    ChainSpec, EthSpecId, ExecutionBlockHash, ForkVersionedResponse, GnosisEthSpec, MainnetEthSpec,
    MinimalEthSpec, PublicKeyBytes, SecretKey, Slot,
};
use std::net::SocketAddr;
use std::sync::Arc;

pub use crate::error::Error;

mod builder;
mod config;
mod error;
mod sse;
mod types;

// TODO: allow other specs to be configured
type E = MainnetEthSpec;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    let sse_listener =
        SseListener::new(config.beacon_node.clone(), config.payload_attributes_cache);
    let secret_key = SecretKey::random();

    let spec = if let Some(ref config_path_str) = config.custom_network {
        let config_path = std::path::Path::new(config_path_str);
        let config = eth2::types::chain_spec::Config::from_file(config_path)
            .expect("Config should be loaded from provided file path");
        match config.eth_spec_id().unwrap_or(EthSpecId::Mainnet) {
            EthSpecId::Mainnet => ChainSpec::from_config::<MainnetEthSpec>(&config),
            EthSpecId::Minimal => ChainSpec::from_config::<MinimalEthSpec>(&config),
            EthSpecId::Gnosis => ChainSpec::from_config::<GnosisEthSpec>(&config),
        }
        .expect("ChainSpec should be constructed from config")
    } else {
        ChainSpec::mainnet()
    };

    let config_name = spec
        .config_name
        .as_ref()
        .map(String::as_str)
        .unwrap_or("Unknown network");
    tracing::info!("loaded chain config: {}", config_name);

    let builder = Arc::new(Builder::new(
        secret_key,
        sse_listener.clone(),
        config.clone(),
        spec,
    ));

    // Spawn event listener on its own thread.
    let sse_handle = tokio::spawn(async move { sse_listener.listen().await });

    let app = Router::new()
        .route("/eth/v1/builder/validators", post(register))
        .route(
            "/eth/v1/builder/header/:slot/:parent_hash/:pubkey",
            get(get_header),
        )
        .route("/eth/v1/builder/blinded_blocks", post(unblind))
        .route("/eth/v1/builder/status", get(status))
        .with_state(builder);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    // Unreachable, but this acts as a sanity check, making sure we invoked the SSE future.
    let () = sse_handle.await.unwrap();
}

pub async fn register() {
    // Don't care about registrations, return 200 OK.
}

pub async fn get_header(
    maybe_user_agent: Option<TypedHeader<UserAgent>>,
    State(builder): State<Arc<Builder>>,
    path: Result<Path<(Slot, ExecutionBlockHash, PublicKeyBytes)>, PathRejection>,
) -> Result<Json<ForkVersionedResponse<SignedBuilderBid<E>>>, (StatusCode, String)> {
    let user_agent =
        maybe_user_agent.map_or("none".to_string(), |agent| agent.as_str().to_string());
    tracing::info!(user_agent, "payload header requested");

    let Path((slot, parent_hash, _)) = path.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid path params: {}", e.body_text()),
        )
    })?;

    match builder.get_header::<E>(slot, parent_hash).await {
        Ok(header) => Ok(Json(header)),
        Err(Error::NoPayload) => Err((StatusCode::NO_CONTENT, "no payload available".into())),
        Err(e) => {
            tracing::warn!(error = ?e, "header request failed");
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")))
        }
    }
}

pub async fn status() {
    // Always healthy.
}

pub async fn unblind() -> (StatusCode, String) {
    // Unblinding is intentionally not implemented. These payloads are not valid.
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "not implemented".to_string(),
    )
}

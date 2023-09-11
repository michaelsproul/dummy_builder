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
    BlindedPayload, ChainSpec, EthSpec, EthSpecId, ExecutionBlockHash, ForkVersionedResponse,
    FullPayloadContents, GnosisEthSpec, MainnetEthSpec, MinimalEthSpec, PublicKeyBytes, SecretKey,
    SignedBlockContents, Slot,
};
use eth2_network_config::Eth2NetworkConfig;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

pub use crate::error::Error;
use crate::payload_cache::PayloadCache;

mod builder;
mod config;
mod error;
mod payload_cache;
mod sse;
mod types;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    let network_config = if let Some(ref config_path_str) = config.custom_network {
        let config_path = std::path::Path::new(config_path_str);
        eth2::types::chain_spec::Config::from_file(config_path)
            .expect("network config should be loaded from provided file path")
    } else {
        let Eth2NetworkConfig { config, .. } = Eth2NetworkConfig::constant(&config.network)
            .and_then(|maybe_config| maybe_config.ok_or("network should exist".to_string()))
            .expect("hardcoded network config file should decode");
        config
    };

    match network_config.eth_spec_id().unwrap_or(EthSpecId::Mainnet) {
        EthSpecId::Mainnet => start_with_config::<MainnetEthSpec>(config, network_config).await,
        EthSpecId::Minimal => start_with_config::<MinimalEthSpec>(config, network_config).await,
        EthSpecId::Gnosis => start_with_config::<GnosisEthSpec>(config, network_config).await,
    }
}

pub async fn start_with_config<E: EthSpec>(
    config: Config,
    network_config: eth2::types::chain_spec::Config,
) {
    let spec = ChainSpec::from_config::<E>(&network_config)
        .expect("ChainSpec should be constructed from config");
    let config_name = spec.config_name.as_deref().unwrap_or("Unknown network");
    tracing::info!("loaded chain config: {}", config_name);

    let sse_listener =
        SseListener::new(config.beacon_node.clone(), config.payload_attributes_cache);
    let secret_key = SecretKey::random();

    let builder = Arc::new(Builder::<E>::new(
        secret_key,
        sse_listener.clone(),
        config.clone(),
        spec,
        PayloadCache::default(),
    ));

    // Spawn event listener on its own thread.
    let sse_handle = tokio::spawn(async move { sse_listener.listen().await });

    let app = Router::new()
        .route("/eth/v1/builder/validators", post(register))
        .route(
            "/eth/v1/builder/header/:slot/:parent_hash/:pubkey",
            get(get_header),
        )
        .route("/eth/v1/builder/blinded_blocks", post(submit_blinded_block))
        .route("/eth/v1/builder/status", get(status))
        .with_state(builder);

    let addr = config
        .listen_address
        .parse::<IpAddr>()
        .expect("listen-address should be a valid Ip address");

    let addr = SocketAddr::new(addr, config.port);
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

pub async fn get_header<E: EthSpec>(
    maybe_user_agent: Option<TypedHeader<UserAgent>>,
    State(builder): State<Arc<Builder<E>>>,
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

    match builder.get_header(slot, parent_hash).await {
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

pub async fn submit_blinded_block<E: EthSpec>(
    maybe_user_agent: Option<TypedHeader<UserAgent>>,
    State(builder): State<Arc<Builder<E>>>,
    Json(block): Json<SignedBlockContents<E, BlindedPayload<E>>>,
) -> Result<Json<ForkVersionedResponse<FullPayloadContents<E>>>, (StatusCode, String)> {
    let user_agent =
        maybe_user_agent.map_or("none".to_string(), |agent| agent.as_str().to_string());
    tracing::info!(user_agent, "signed blinded block received");

    match builder.submit_blinded_block(block).await {
        Ok(full_payload_contents) => Ok(Json(full_payload_contents)),
        Err(e) => {
            tracing::warn!(error = ?e, "submit blinded block request failed");
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")))
        }
    }
}

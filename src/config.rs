use clap::Parser;
use eth2::types::Uint256;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Parser)]
#[command(about = "Ethereum dummy block builder")]
pub struct Config {
    /// Beacon node to stream payload attributes from.
    #[arg(long, value_name = "URL", default_value = "http://localhost:5052")]
    pub beacon_node: String,

    /// Port to listen on.
    #[arg(long, value_name = "N", default_value = "18550")]
    pub port: u16,

    /// Number of payload attributes to cache in memory.
    #[arg(long, value_name = "N", default_value = "16")]
    pub payload_attributes_cache: NonZeroUsize,

    /// Number of zero bytes used to pad the transactions field of the payload.
    #[arg(long, value_name = "N", default_value = "0")]
    pub payload_body_bytes: usize,

    /// Value of the payload in WEI.
    #[arg(long, value_name = "N", default_value = "0")]
    pub payload_value: Uint256,

    /// The network to use. Defaults to mainnet.
    #[arg(long, value_name = "NAME", default_value = "mainnet")]
    pub network: String,

    /// The custom network config to use. Overrides the --network flag
    #[arg(long, value_name = "PATH")]
    pub custom_network: Option<String>,
}

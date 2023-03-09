use eth2::types::{
    ChainSpec, EthSpec, ExecutionPayloadHeader, PublicKey, SecretKey, Signature, SignedRoot,
    Uint256 as U256,
};
use serde::Serialize;
use tree_hash_derive::TreeHash;

#[derive(Serialize, TreeHash)]
pub struct Bid<E: EthSpec> {
    pub header: ExecutionPayloadHeader<E>,
    #[serde(with = "serde_utils::quoted_u256")]
    pub value: U256,
    pub pubkey: PublicKey,
}

impl<E: EthSpec> SignedRoot for Bid<E> {}

impl<E: EthSpec> Bid<E> {
    #[must_use]
    pub fn sign(&self, secret_key: &SecretKey, spec: &ChainSpec) -> Signature {
        let domain = spec.get_builder_domain();
        let message = self.signing_root(domain);
        secret_key.sign(message)
    }
}

#[derive(Serialize)]
pub struct SignedBid<E: EthSpec> {
    pub message: Bid<E>,
    pub signature: Signature,
}

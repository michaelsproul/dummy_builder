use eth2::types::{
    ChainSpec, EthSpec, ExecutionPayloadHeader, ForkName, PublicKey, SecretKey, Signature,
    SignedRoot, Uint256 as U256,
};
use serde::Serialize;
use tree_hash_derive::TreeHash;

#[derive(Serialize, TreeHash)]
pub struct Bid<E: EthSpec> {
    pub header: ExecutionPayloadHeader<E>,
    pub value: U256,
    pub public_key: PublicKey,
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
pub struct SignedVersionedResponse<T> {
    pub version: ForkName,
    pub data: T,
    pub signature: Signature,
}

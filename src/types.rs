use eth2::types::builder_bid::BuilderBid;
use eth2::types::{ChainSpec, EthSpec, SecretKey, Signature, SignedRoot};

pub trait SignableBid {
    fn sign(&self, secret_key: &SecretKey, spec: &ChainSpec) -> Signature;
}

impl<E: EthSpec> SignableBid for BuilderBid<E> {
    #[must_use]
    fn sign(&self, secret_key: &SecretKey, spec: &ChainSpec) -> Signature {
        let domain = spec.get_builder_domain();
        let message = self.signing_root(domain);
        secret_key.sign(message)
    }
}

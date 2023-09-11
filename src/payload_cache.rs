use eth2::types::{EthSpec, FullPayloadContents, Hash256};
use lru::LruCache;
use std::num::NonZeroUsize;
use tokio::sync::Mutex;
use tree_hash::TreeHash;

pub const DEFAULT_PAYLOAD_CACHE_SIZE: usize = 10;

/// A cache mapping execution payloads by tree hash roots.
pub struct PayloadCache<E: EthSpec> {
    payloads: Mutex<LruCache<PayloadCacheId, FullPayloadContents<E>>>,
}

#[derive(Hash, PartialEq, Eq)]
struct PayloadCacheId(Hash256);

impl<E: EthSpec> Default for PayloadCache<E> {
    fn default() -> Self {
        PayloadCache {
            payloads: Mutex::new(LruCache::new(
                NonZeroUsize::new(DEFAULT_PAYLOAD_CACHE_SIZE).unwrap(),
            )),
        }
    }
}

impl<E: EthSpec> PayloadCache<E> {
    pub async fn put(&self, payload: FullPayloadContents<E>) -> Option<FullPayloadContents<E>> {
        let root = payload.payload_ref().tree_hash_root();
        self.payloads
            .lock()
            .await
            .put(PayloadCacheId(root), payload)
    }

    pub async fn pop(&self, hash: &Hash256) -> Option<FullPayloadContents<E>> {
        self.payloads.lock().await.pop(&PayloadCacheId(*hash))
    }
}

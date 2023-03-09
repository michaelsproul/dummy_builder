//! Listen for payload attributes on the SSE endpoint of a beacon node and cache them.
use eth2::types::{ExecutionBlockHash, Slot, VersionedSsePayloadAttributes};
use eventsource_client::{Client, ClientBuilder, SSE};
use futures::TryStreamExt;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct SseListener {
    /// Beacon node URL.
    pub url: String,
    /// Map from (parent_hash, slot) to payload attributes.
    pub payload_attributes:
        Arc<Mutex<LruCache<(ExecutionBlockHash, Slot), VersionedSsePayloadAttributes>>>,
}

impl SseListener {
    pub fn new(url: String, cache_size: NonZeroUsize) -> Self {
        Self {
            url,
            payload_attributes: Arc::new(Mutex::new(LruCache::new(cache_size))),
        }
    }

    pub async fn listen(self) {
        tracing::info!("starting SSE listener");

        let url = format!("{}/eth/v1/events?topics=payload_attributes", self.url);
        let client = ClientBuilder::for_url(&url).unwrap().build();

        if let Err(e) = Box::pin(client.stream())
            .try_for_each(|ev| async {
                match ev {
                    SSE::Event(event) => {
                        let payload_attributes: VersionedSsePayloadAttributes =
                            match serde_json::from_str(&event.data) {
                                Ok(p) => p,
                                Err(e) => {
                                    tracing::error!("invalid payload attributes: {:?}", e);
                                    return Ok(());
                                }
                            };
                        let parent_block_hash = payload_attributes.data.parent_block_hash;
                        let slot = payload_attributes.data.proposal_slot;

                        tracing::info!(
                            parent = ?parent_block_hash,
                            slot = %slot,
                            "got new payload attributes"
                        );

                        self.payload_attributes
                            .lock()
                            .await
                            .put((parent_block_hash, slot), payload_attributes);
                        Ok(())
                    }
                    SSE::Comment(_) => Ok(()),
                }
            })
            .await
        {
            tracing::error!("stream terminated with error: {:?}", e);
        }
    }

    pub async fn get_payload_attributes(
        &self,
        parent_hash: ExecutionBlockHash,
        slot: Slot,
    ) -> Option<VersionedSsePayloadAttributes> {
        self.payload_attributes
            .lock()
            .await
            .get(&(parent_hash, slot))
            .cloned()
    }
}

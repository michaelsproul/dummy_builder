use crate::payload_cache::PayloadCache;
use crate::types::SignableBid;
use crate::{config::Config, sse::SseListener, Error};
use eth2::types::builder_bid::{
    BlindedBlobsBundle, BuilderBid, BuilderBidCapella, BuilderBidDeneb, BuilderBidMerge,
    SignedBuilderBid,
};
use eth2::types::{
    BlindedPayload, BlobsBundle, ChainSpec, EthSpec, ExecutionBlockHash, ExecutionPayload,
    ExecutionPayloadCapella, ExecutionPayloadDeneb, ExecutionPayloadHeader, ExecutionPayloadMerge,
    ForkName, ForkVersionedResponse, FullPayloadContents, PublicKey, SecretKey,
    SignedBlockContents, Slot, Uint256, VariableList,
};
use tree_hash::TreeHash;

pub struct Builder<E: EthSpec> {
    sse_listener: SseListener,
    public_key: PublicKey,
    secret_key: SecretKey,
    config: Config,
    spec: ChainSpec,
    payload_cache: PayloadCache<E>,
}

impl<E: EthSpec> Builder<E> {
    pub fn new(
        secret_key: SecretKey,
        sse_listener: SseListener,
        config: Config,
        spec: ChainSpec,
        payload_cache: PayloadCache<E>,
    ) -> Self {
        let public_key = secret_key.public_key();
        Self {
            sse_listener,
            public_key,
            secret_key,
            config,
            spec,
            payload_cache,
        }
    }

    pub async fn get_header(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
    ) -> Result<ForkVersionedResponse<SignedBuilderBid<E>>, Error> {
        let ext_payload_attributes = self
            .sse_listener
            .get_payload_attributes(parent_hash, slot)
            .await
            .ok_or(Error::NoPayload)?;
        let payload_attributes = &ext_payload_attributes.data.payload_attributes;

        let pubkey = self.public_key.clone();
        let secret_key = &self.secret_key;

        let fee_recipient = payload_attributes.suggested_fee_recipient();
        let timestamp = payload_attributes.timestamp();
        let prev_randao = payload_attributes.prev_randao();
        let block_number = ext_payload_attributes.data.parent_block_number + 1;
        let gas_limit = 30_000_000;

        let transactions = VariableList::new(vec![VariableList::new(vec![
            0;
            self.config
                .payload_body_bytes
        ])
        .unwrap()])
        .unwrap();
        let value = self.config.payload_value;

        let version = ext_payload_attributes.version.ok_or(Error::LogicError)?;

        // Using a dummy block hash as we don't need a valid payload, but LH expects a non zero hash.
        let block_hash = ExecutionBlockHash::repeat_byte(42);
        let (payload, maybe_blobs) = match version {
            ForkName::Merge => (
                ExecutionPayload::Merge(ExecutionPayloadMerge {
                    parent_hash,
                    timestamp,
                    fee_recipient,
                    prev_randao,
                    block_number,
                    gas_limit,
                    transactions,
                    block_hash,
                    ..Default::default()
                }),
                None,
            ),
            ForkName::Capella => {
                let withdrawals = payload_attributes
                    .withdrawals()
                    .map_err(|_| Error::LogicError)?
                    .clone()
                    .into();
                (
                    ExecutionPayload::Capella(ExecutionPayloadCapella {
                        parent_hash,
                        timestamp,
                        fee_recipient,
                        prev_randao,
                        block_number,
                        gas_limit,
                        transactions,
                        withdrawals,
                        block_hash,
                        ..Default::default()
                    }),
                    None,
                )
            }
            ForkName::Deneb => {
                let withdrawals = payload_attributes
                    .withdrawals()
                    .map_err(|_| Error::LogicError)?
                    .clone()
                    .into();
                (
                    ExecutionPayload::Deneb(ExecutionPayloadDeneb {
                        parent_hash,
                        timestamp,
                        fee_recipient,
                        prev_randao,
                        block_number,
                        gas_limit,
                        transactions,
                        withdrawals,
                        block_hash,
                        ..Default::default()
                    }),
                    Some(BlobsBundle::default()),
                )
            }
            _ => return Err(Error::NoPayload),
        };

        let header = ExecutionPayloadHeader::from(payload.clone().to_ref());
        let bid = new_dummy_bid(header, value, pubkey);
        let signature = bid.sign(secret_key, &self.spec);

        self.payload_cache
            .put(FullPayloadContents::new(payload, maybe_blobs))
            .await;

        Ok(ForkVersionedResponse {
            version: Some(version),
            data: SignedBuilderBid {
                message: bid,
                signature,
            },
        })
    }

    pub async fn submit_blinded_block(
        &self,
        signed_blinded_block_contents: SignedBlockContents<E, BlindedPayload<E>>,
    ) -> Result<ForkVersionedResponse<FullPayloadContents<E>>, Error> {
        let block_hash = signed_blinded_block_contents
            .signed_block()
            .message()
            .execution_payload()
            .map_err(|_| Error::LogicError)?
            .tree_hash_root();

        let full_payload_contents = self
            .payload_cache
            .pop(&block_hash)
            .await
            .ok_or(Error::UnbindPayloadError(block_hash))?;

        let fork = signed_blinded_block_contents
            .signed_block()
            .fork_name_unchecked();

        Ok(ForkVersionedResponse {
            version: Some(fork),
            data: full_payload_contents,
        })
    }
}

fn new_dummy_bid<E: EthSpec>(
    payload: ExecutionPayloadHeader<E>,
    value: Uint256,
    pubkey: PublicKey,
) -> BuilderBid<E> {
    match payload {
        ExecutionPayloadHeader::Merge(header) => BuilderBid::Merge(BuilderBidMerge {
            header,
            value,
            pubkey: pubkey.into(),
        }),
        ExecutionPayloadHeader::Capella(header) => BuilderBid::Capella(BuilderBidCapella {
            header,
            value,
            pubkey: pubkey.into(),
        }),
        ExecutionPayloadHeader::Deneb(header) => BuilderBid::Deneb(BuilderBidDeneb {
            header,
            blinded_blobs_bundle: BlindedBlobsBundle::default(),
            value,
            pubkey: pubkey.into(),
        }),
    }
}

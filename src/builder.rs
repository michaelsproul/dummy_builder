use crate::types::SignableBid;
use crate::{config::Config, sse::SseListener, Error};
use eth2::types::beacon_block_body::KzgCommitments;
use eth2::types::builder_bid::{
    BlindedBlobsBundle, BuilderBid, BuilderBidCapella, BuilderBidDeneb, BuilderBidMerge,
    SignedBuilderBid,
};
use eth2::types::{
    BlobRootsList, ChainSpec, EthSpec, ExecutionBlockHash, ExecutionPayload,
    ExecutionPayloadCapella, ExecutionPayloadDeneb, ExecutionPayloadHeader, ExecutionPayloadMerge,
    ForkName, ForkVersionedResponse, KzgProofs, PublicKey, SecretKey, Slot, Uint256, VariableList,
};

pub struct Builder {
    sse_listener: SseListener,
    public_key: PublicKey,
    secret_key: SecretKey,
    config: Config,
    spec: ChainSpec,
}

impl Builder {
    pub fn new(
        secret_key: SecretKey,
        sse_listener: SseListener,
        config: Config,
        spec: ChainSpec,
    ) -> Self {
        let public_key = secret_key.public_key();
        Self {
            sse_listener,
            public_key,
            secret_key,
            config,
            spec,
        }
    }

    pub async fn get_header<E: EthSpec>(
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

        let payload = match version {
            ForkName::Merge => ExecutionPayload::Merge(ExecutionPayloadMerge {
                parent_hash,
                timestamp,
                fee_recipient,
                prev_randao,
                block_number,
                gas_limit,
                transactions,
                ..Default::default()
            }),
            ForkName::Capella => {
                let withdrawals = payload_attributes
                    .withdrawals()
                    .map_err(|_| Error::LogicError)?
                    .clone()
                    .into();
                ExecutionPayload::Capella(ExecutionPayloadCapella {
                    parent_hash,
                    timestamp,
                    fee_recipient,
                    prev_randao,
                    block_number,
                    gas_limit,
                    transactions,
                    withdrawals,
                    ..Default::default()
                })
            }
            ForkName::Deneb => {
                let withdrawals = payload_attributes
                    .withdrawals()
                    .map_err(|_| Error::LogicError)?
                    .clone()
                    .into();
                ExecutionPayload::Deneb(ExecutionPayloadDeneb {
                    parent_hash,
                    timestamp,
                    fee_recipient,
                    prev_randao,
                    block_number,
                    gas_limit,
                    transactions,
                    withdrawals,
                    ..Default::default()
                })
            }
            _ => return Err(Error::NoPayload),
        };

        let header = ExecutionPayloadHeader::from(payload.to_ref());

        let bid = new_dummy_bid(header, value, pubkey);
        let signature = bid.sign(secret_key, &self.spec);

        Ok(ForkVersionedResponse {
            version: Some(version),
            data: SignedBuilderBid {
                message: bid,
                signature,
            },
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
            blinded_blobs_bundle: empty_blinded_blobs_bundle(),
            value,
            pubkey: pubkey.into(),
        }),
    }
}

fn empty_blinded_blobs_bundle<E: EthSpec>() -> BlindedBlobsBundle<E> {
    BlindedBlobsBundle {
        commitments: KzgCommitments::<E>::empty(),
        proofs: KzgProofs::<E>::empty(),
        blob_roots: BlobRootsList::<E>::empty(),
    }
}

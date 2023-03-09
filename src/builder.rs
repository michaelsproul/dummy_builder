use crate::{
    sse::SseListener,
    types::{Bid, SignedBid},
    Error,
};
use eth2::types::{
    ChainSpec, EthSpec, ExecutionBlockHash, ExecutionPayload, ExecutionPayloadCapella,
    ExecutionPayloadHeader, ExecutionPayloadMerge, ForkName, ForkVersionedResponse, PublicKey,
    SecretKey, Slot, Uint256 as U256,
};

pub struct Builder {
    sse_listener: SseListener,
    public_key: PublicKey,
    secret_key: SecretKey,
    spec: ChainSpec,
}

impl Builder {
    pub fn new(secret_key: SecretKey, sse_listener: SseListener, spec: ChainSpec) -> Self {
        let public_key = secret_key.public_key();
        Self {
            sse_listener,
            public_key,
            secret_key,
            spec,
        }
    }

    pub async fn get_header<E: EthSpec>(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
    ) -> Result<ForkVersionedResponse<SignedBid<E>>, Error> {
        let ext_payload_attributes = self
            .sse_listener
            .get_payload_attributes(parent_hash, slot)
            .await
            .ok_or(Error::NoPayload)?;
        let payload_attributes = &ext_payload_attributes.data.payload_attributes;

        let public_key = self.public_key.clone();
        let secret_key = &self.secret_key;

        let fee_recipient = payload_attributes.suggested_fee_recipient();
        let timestamp = payload_attributes.timestamp();
        let prev_randao = payload_attributes.prev_randao();
        let block_number = ext_payload_attributes.data.parent_block_number + 1;
        let gas_limit = 30_000_000;

        let value = U256::from(1);

        let version = ext_payload_attributes.version.ok_or(Error::LogicError)?;

        let payload = match version {
            ForkName::Merge => ExecutionPayload::Merge(ExecutionPayloadMerge {
                parent_hash,
                timestamp,
                fee_recipient,
                prev_randao,
                block_number,
                gas_limit,
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
                    withdrawals,
                    ..Default::default()
                })
            }
            _ => return Err(Error::NoPayload),
        };

        let header = ExecutionPayloadHeader::from(payload.to_ref());

        let bid = Bid {
            header,
            value,
            public_key,
        };
        let signature = bid.sign(&secret_key, &self.spec);

        Ok(ForkVersionedResponse {
            version: Some(version),
            data: SignedBid {
                message: bid,
                signature,
            },
        })
    }
}

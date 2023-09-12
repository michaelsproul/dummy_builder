`dummy_builder`
==============

World's dumbest block builder for Ethereum.

This is a block builder designed with very specific goals, it aims to:

- Return a payload header and un-blinded payload as quickly as possible.
- Satisfy all consensus client safety checks on the payload header, e.g. correct randao,
  withdrawals, bid value, etc.
- Minimise dependencies and implementation complexity (depends on Lighthouse but not `mev-rs`).

Non-goals:

- Creating valid payload bodies (!!).

It is designed for use in the `blockprint` ecosystem where we want a fleet of CL nodes to build
a block as quickly as possible with no concern for execution payload validity or quality.

It can also be used for testing block proposal with the builder/mev flow, and should cover the integration points. 
It's worth noting that the payload will contain an invalid block hash, and the published block will always be rejected
by the EL on `newPayload`.

## Quick Start with Docker

`dummy_builder` utilises the BN to produce payload attributes every slot, e.g. if you're running a Lighthouse beacon 
node, you'll need to run Lighthouse with `--always-prepare-payload` and `--prepare-payload-lookahead 8000` to get the BN 
to produce payload attributes every slot (at 4s). 

To build the Docker image:

```bash
docker build -t dummy_builder .
```

To start:

```bash
docker run -p 18550:18550 -e RUST_LOG=info -v $(pwd)/config.yaml:/app/config.yaml dummy_builder \
  --beacon-node http://docker.host.internal:5052 \
  --payload-value 20000000000000000 \
  --custom-network /app/config.yaml \
  --listen-address 0.0.0.0
```  
  
This starts `dummy_builder` on http://localhost:18550. Run `dummy_builder --help` to see all available options.

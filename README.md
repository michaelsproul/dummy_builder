`dummy_builder`
==============

World's dumbest block builder for Ethereum.

This is a block builder designed with very specific goals, it aims to:

- Return a payload header as quickly as possible.
- Satisfy all consensus client safety checks on the payload header, e.g. correct randao,
  withdrawals, bid value, etc.
- Minimise dependencies and implementation complexity (depends on Lighthouse but not `mev-rs`).

Non-goals:

- Creating valid payload bodies (!!). Unblinding the headers is not implemented.

It is designed for use in the `blockprint` ecosystem where we want a fleet of CL nodes to build
a block as quickly as possible with no concern for execution payload validity or quality.

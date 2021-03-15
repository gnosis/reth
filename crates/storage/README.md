# reth Storage
w
This crate is responsible for implementing all storage and storage filling mechanism used within reth. There are two primary types of storage:

  - Blockchain storage, responsible for storing a copy of all known blocks and transactions.
  - State storage, responsible for storing the world state (the distributed ledger and contract state).

It should be able to act as standalone binary that allows modification on database in offline mode: offline prunning,inspection of block/state, making offline snapshot, importing snapshot from folder, upgrades, db integrity checks, etc..

## Design notes
  - Flat-DB, cache merkle-tree hashes on intermediate nodes.
  - Prunning and compression,
  - Marking regions of storage that are accessed by certain contracts and applying pruning/locality/paging policies.
  - This crate should have dependency only on the `core` create and is populated by applying `StateDiff`s.
  - Storage have caches for things such as: `nonce`s, recent state, etc.

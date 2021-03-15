# Reth

Reth 1.0 - Planning &amp; Design Repository

This repository is used to create a blueprint of the system design for the new reth edition. At the moment it is used to gather the most important high-level design decisions.

Please browse through individual crates for more specific discussions and/or design decisions:

  - [Core](crates/core/README.md) (fundamental types)
  - [Importer](crates/importer/README.md) (EVM, Miner)
  - [Networking](crates/networking/README.md) (devp2p, json-rpc)
  - [Storage](crates/storage/README.md) (snapshotting, import/export, state, blocks store, pruning, archival, etc.)
  - [Consensus](crates/consensus/README.md) (Protocols, PoW , Clique, PoS, AuRa, etc..)
  - [Transaction Pool](crates/txpool/README.md)
  - [Scheduler](crates/scheduler/README.md) (BlockSync, Peers)
  - [Tests](crates/tests/README.md) (Integration and eth tests)

Binaries:
  - [reth](bin/reth/README.md) 

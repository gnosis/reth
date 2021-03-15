# reth core

This crate contains the most fundamental ethereum types and should have no dependencies on any other crates in this project.

Its purpose is to define a unified type model for common data types used across various components of the system.

It is expected that almost all other crates will have this one as a dependency.


## Design notes:

  - This crate implements most used structures in Ethereum.

  - All serialization to and from JSON, RLP (or the proposed Blob Serialization).

  - Currently exported types are 
    - large ints: `U128`, `U256`, `U512` 
    - hashes: `H128`, `H160`, `H256`, `H264`, `H512`
    - fundamental concepts: `Block`, `Receipt`, `Transaction`, _ Please do your best not to duplicate those types. If you feel a strong urge to create a new fundamental concept type please ask around the team first._

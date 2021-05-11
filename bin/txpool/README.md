# reth Main Executable

This crate is the main executable of the system and ties together all other crates. This is where `core`, `storage`, `execution`, and `networking` are integrated. This crate also implements the system-level integrations such as the daemon interface and command line parsing.

## Design notes
  - should not be referenced by any other crate.
  - Does not defined any new types that are fundamental concepts.
  - Defines types such as `EnvironmentConfig`, `*Config`, etc.
  - tbd.

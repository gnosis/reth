# reth importer

This crate contains types that implement VMs supported by Ethereum. At the time of writing there are two types of VMs to known to the Ethereum world: EVM and pWASM. pWASM support is fading away and is only supported on testsnets in limited capcity, so the plan moving forward is only to focus on one VM which is EVM.

## Design notes:
- The EVM implementation shoudl operate on state diffs and scoped contract storage. We should not have to provide any instance of EVM types with a view of the entire state storage.
- The input to an EVM instance should be: 
  - Contract bytecode
  - Contract local storage tree
  - Current environement (Block information)
  - Transaction info (sender, etc.)
- The output of invoking the EVM instance should be:
  - `StateDiff`: a collection of storage keys and their values that changed as a result of executing a transaction against a contract. Applying the resulting `StateDiff` to the world state would result in an updated storage root.
  
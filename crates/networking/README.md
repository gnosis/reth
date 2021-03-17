# OpenEthereum 4.0 Networking

This crate implements network IO. There are currently two major servers that implement Wire protocols(devp2p) and JSONRPC protocols (http,websocket).

## Design notes for devp2p

- This crate should not have dependencies on anything other than the `core` crate.
- Most of the time this library will expose _async_ `Stream`s of things, such as:

  - ```rust 
    let network = open_network(...)?;
    while let Some(transaction) = network.transactions.next().await {
      // insert transaction into local transaction pool
      transactions_queue.append(transaction);
    }
    ```
  - ```rust 
    let network = open_network(...)?;
    while let Some(peer) = network.peers.next().await {
      // add peer to local peers or otherwise process it
      if let Ok(handshake) = syncpeers.handshake(peer)? {
        // connection established
      }
    }
    ```  
  - ```rust 
    let network = open_network(...)?;
    while let Some(block) = network.blocks.next().await {
      // import new mined block
      storage.chain.append(block)?;
    }
    ```  
- Design this crate in a way that makes it possible for use in scenarios such as:
  - Writing a tool that enumerates all known peers and return their client versions and supported protocols in as few lines of code as possible.
  - Writing a tool that imports blocks across sync protocol (warp, fastsync, etc.) for faster sync with multiple clients.
  - etc.
  - IF you think about those scenarios and how easy it would be to use this library to build those tools, that should help gauge whether the library is decoupled enough from the rest of the workspace.

## Design notes for JSONRPC

Endpoints needs to be grouped by functionality and it should be possible to enable/disable them.
Protocol that we can support are http,websocket and ipc.

- There is standardization effort to standardize all JSONRPC calls for Ethereum: https://github.com/ethereum-oasis/eth1.x-JSON-RPC-API-standard
- JSONRPC implemented by OpenEthereum can be found here: https://openethereum.github.io/JSONRPC
- Replace parity-ws with hyper: https://hyper.rs

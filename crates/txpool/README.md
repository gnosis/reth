# reth transaction pool

Implementation of Ethereum transaction pool. It contains transactions pending for inclusion in the block. In the future, it can become a lot smarter and allow programmed inclusion of transactions.

It uses one `BinaryHeap` to sort all transactions and additional helper structures for indexing(`by_hash`, `by_account`). Removed transaction hashes are saved in `for_removal` list and when `BinaryHeap` is recreated they are removed.

Features that pool supports:
* Limit the maximum number of transactions. Configurable
* Limit number of transactions per account. Configurable
* Reject transaction with smaller nonce than currently greatest.
* Reject transaction whose account does not have enough balance to fund its `gas_limit*gas_price`.
* Accept transactions with a gap in the nonce.
* Replace transaction with the same account and nonce but new tx needs to have `12.5%` better score.
* All transactions have a timestamp, and the pool periodically cleanses old transactions.
* Ability to get all transactions sorted by score and nonce. Needed for creation of the pending block.
* Ability to remove transactions after a new block is inserted or reinsert them on a reverted block after chain reorg happens.
* Register announcer for insert/remove announcements.


TODO:
* Limit pool by memory.
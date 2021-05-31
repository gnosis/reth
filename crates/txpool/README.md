# reth transaction pool

Implementation of Ethereum transaction pool. It contains transactions pending for inclusion in the block. In the future, it can become a lot smarter and allow programmed inclusion of transactions.

Features that pool supports:
1. Limit the maximum number of transactions. Configurable
2. When max limit is hit, eject transaction with lowest score  
3. Limit number of transactions per account. Configurable
4. Reject transaction with smaller nonce than current greatest.
5. Reject transaction whose account does not have enough balance to fund it.
6. Accept transactions with a gap in the nonce.
7. Replace transaction with the same account and nonce but new transaction needs to have `12.5%` better score.
8. All transactions have a timestamp, and the pool periodically cleanses old transactions.
9. Ability to get all transactions sorted by score and nonce. Needed for creation of the pending block.
10. Ability to remove transactions after a new block is inserted or reinsert them on a reverted block after chain reorg happens.
11. Register announcer to track all inserted/removed transactions.


TODO:
* Limit pool by memory.
* Allow to configure `local` accounts where limit per account will not be affected. Bear in mind that it still can be ejected if it has the lowest score (gas_price/tip).

All functionality function as constraints on structures that we are using, as soo we are saving pointer of transaction in multiple ways depending on what we need. Those different ways are:
* `by_hash` (`HashMap<Keccak,Tx>`) used for fast lookup of transaction.
* `by_account` (`HashMap<Address, Account>`) used to find account that has transaction in pool. `Account` has:
    * Nonce and Balance: taken from world state. This is updated on every new block insertion, reorg or it is obtained when we try to insert new transaction into pool.
    * List of transactions, sorted by nonce (Vec<Tx>), as said, there could be nonce gaps but number of transaction per account is limited, this is done as security requirment to stop spamming. 
* `by_score` (`BinaryHeap`) structure used for sorting of transactions for pending block and it give us ability to eject lowest scored transaction. Removed transaction hashes are temporarily saved in `for_removal` list and when `BinaryHeap` is recreated they are removed. In this case, it is BinaryHeap but in general, we could use BTreeMap or any other structure that allow fast insertion of items in sorted order.


For block update for both reverted (reorg) or new included blocks we require all account changes (nonce and balance) and list of reverted transaction that needs to be reincluded into pool.
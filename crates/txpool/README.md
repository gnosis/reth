# reth transaction pool

Implementation of Ethereum transaction pool. It contains transactions pending for inclusion in the block. In the future, it can become a lot smarter and allow programmed inclusion of transactions. There are three main parts inside txpool: handle API from sentry(p2p) with eth/65 protocol, storage (txpool) where tx is saved and ordered, miner whose job is to prepare txs for pending block(execute them, fill pending block, communicate with outside miner/sealer)

## Features that txpool supports:
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

Additional TODO for txpool:
* Limit pool by memory.
* Allow to configure `local` accounts where limit per account will not be affected. Bear in mind that it still can be ejected if it has the lowest score (gas_price/tip).
* Bundle update block revert with new block. With reorg happening we are allways going to receive one or more reverted blocks following it with one or more new blocks. This can potentially disturb pool because of reverted block contains reverted txs are going to squeeze out a lot of txs and they are probably going to be remove. Potential solution is to buffer those reverted blocks and when one new block comes wait for a little for potentially more blocks, and with bufferer blocks changes, bundle all account changes and reverted tx and update the txpool.  

## Pending discussions:
* If we have multiple tx from same account and tx with lowest nonce has lowest score and it needs to be ejected, by removing this tx we will be making nonce gaps, and this will make other txs not usable. Possible sulution is to not remove this tx but remove tx from same account but with gratest nonce.
*  discuss if this use case needs to be covered for inclusion of txs in pending block: If we have tx0 and tx1 from same author with nonces 0 and 1, and tx1 has better score then tx0, that would mean that when iterating we are going to skip tx1 and include only tx0. Should we tranverse back and try to include tx1 again? edge case: if including tx1 removes some tx from pending block (or even removes tx0 ).
* Currently possible attack vector is to create multiple tx from different account but with nonce gaps and with that flood the pool, we need to somehow limit gaps in nonces. We should support gaps because order of received txs can be differrent and can potentially delay inclusion of txs with lower nonce. But if we allow infinity number of gaps, txpool can potentially become stuck. Maybe somehow allow faster time based purging of accounts or txs with nonce gaps? Or maybe limit number of accounts that can have gaps? Not sure what is the best way.

## Pool structures
All functionality function as constraints on structures that we are using, as soo we are saving pointer of transaction in multiple ways depending on what we need. Those different ways are:
* `by_hash` (`HashMap<Keccak,Tx>`) used for fast lookup of transaction.
* `by_account` (`HashMap<Address, Account>`) used to find account that has transaction in pool. `Account` has:
    * Nonce and Balance: taken from world state. This is updated on every new block insertion, reorg or it is obtained when we try to insert new transaction into pool.
    * List of transactions, sorted by nonce (Vec<Tx>), as said, there could be nonce gaps but number of transaction per account is limited, this is done as security requirment to stop spamming. 
* `by_score` (`BinaryHeap`) structure used for sorting of transactions for pending block and it give us ability to eject lowest scored transaction. Removed transaction hashes are temporarily saved in `for_removal` list and when `BinaryHeap` is recreated they are removed. In this case, it is BinaryHeap but in general, we could use BTreeMap or any other structure that allow fast insertion of items in sorted order. There is proposal for eip1559 to split BinaryHeap in two, one with `effective_gas_price` that is more then `base_fee` and second one for tx with lowe `effective_gas_price` then `base_fee`, generally speaking this is optimization that probably is not needed because one binary heap is good enought for resorting even if we have 50k tx in pool (but to be sure about this statement, it is best to test it) .

## Peers (Sentry p2p handlers)
This part implement handlers for all inbound messages from p2p network (sentry). Here we have list of peers that needs to have HashSet of known transactions, and it needs to handle fetching txs data when txs hash is received from network. On other side it connects to txpool for three functiona: find/get tx, include tx and on_new_tx for notification for new tx. This is one of possible vector of attack because it connects dirrectly to p2p network. Some behaviour that we need to be aware of:
* when receiving **NewTxPoolHashes** firstly we need to check if we already have these txs (maybe somehow penalize if peer keeps sending known tx). Additionaly we can expect to receive same tx hash from multiple peers and we could have somekind of mechanism that does not make more then N requests to multiple peers for same data, maybe just add some delay as we have seen that **geth fetcher** do. After that make request with **GetPooledTxs** and wait for response. Naive approche is to allways ask for unknown txs, this is unoptimized sollution but can still work.
* When receiving **PooledTxs** we need to check if response match tx hashes that we have requested with **GetPooledTxs** and if everything is okay **include** new tx to pool. for **Txs** msg, just try to **include** them in pool
* When receiving **GetPooledtxs** simple find them in pool and responde to request as best as we can.
* For **brodcasting** of new txs, for every peer we need to check if txs that we want to send are known to him and send only unknown.

For more information please see [eth/65 spec](https://eips.ethereum.org/EIPS/eip-2464)

## Miner
It is responsible to take sorted txs from txpool execute them to get gas spend and with that fill pending block header with all needed fields (state root hash, transaction root, etc..). With pending block done, send its hash externally for mining or to AuRa/Clique for consensus part sealing. Miner should contains EVM and read only connection to db. For eth2 miner should be responsible to create pending block for eth2 client and maybe receive sealed eth2 block for inspection and inclusion (or should eth2 contact core directly?)

Discuss: how often should we recreate pending block to include new txs added to pool? Should miner ask txpool for txs or txpool will stream changes to miner if there is for example more then 10% change in sorting of tx.

Maybe there should be RPC support for pending block/transactions so that for example users can get traces for still pending block?  
## Block update/reorg
For block update for both reverted (reorg) or new included blocks we require all account changes (nonce and balance) and list of reverted transaction that needs to be reincluded into pool.

## Predictive execution of block txs
One of potential big speedup of new block inclusion can be gained by predicting execution of its transactions. All clients if they are preparing pending block are executing most of transactions at least two times. First time when creating pending block to check validiti of txs and how much gas is spend, and second time when new blocks comes from p2p network for inclusion. That of course can be different block but it is assumed that that miner chose txs from txpool and it did sorting closely as we are doing it in our txpool. Here we have potential to use first txpool execution to predict execution of blocks txs. This is ispired by hyperledger fabric and its execute/order/validate way of inclusion, on first txpool execution we can save block hash, addresses of read accounts (and data) that we will use for verification and all data writen to db after execution. Second step is when new block comes for inclusion we can check if we have preexecuted txs hash, check if accounts that we want to read are dirty (if data matches) or not, and then just apply writes that we got from first execution and with that skip executing it second time.

For advance research topic, if we know what contracts types are called from what transaction from first execution, could we predict how connected they are, maybe there is possibility to paralalize execution second time?
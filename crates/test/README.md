# reth transaction pool

Implementation of ethereum transaction pool. It contains transaction pending to inclusion in block. In future is can become a lot smarter and allow programed inclusion of transaction.

It is in memory structure and it should be fast. It should allow filtering and ordering of transaction by their score (gas price)

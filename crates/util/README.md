# Rust Ethereum utility library 

Purpose is to group parts of code that can be used for rust ethereum client.

Currently it contains:
* Queue:
  * AsyncQueue: VecDeque that can be used to send Item from one thread to another. Max items and batches execution are supported.
  * ExecutionQueue: It spins one thread and uses closure to executes Item for queue. It uses AsyncQueue as queue.

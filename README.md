# solana-rpc-benchmark

We want to validate the performance of different Solana RPC nodes.

For that, we will write a program, that accepts a list of Solana RPC
endpoints and creates a thread for each of them.

In each thread, we then use the individual RPC endpoint to
create a new transaction with the amount of 0.000000001 SOL
send to an address provided via a CLI argument to the program.

The program should accept the following parameters:
- Solana keypair.json to send the transaction from
- Public Solana Wallet address to send the transaction to
- A comma separated list of RPC endpoints provided as a string
- The amount to send per transaction which defaults 0.000000001 SOL

To measure the performance, we want to log the following per thread and transaction:

- created at time for each thread
- rpc endpoint used by the current thread
- current block height of the RPC node
- The transaction signature for each transaction
- The block height for each transaction
- All metadata for each transaction
- time at the end of each thread

```
cargo run \
  -- \
  --endpoints "https://api.mainnet-beta.solana.com,https://rpc.ankr.com/solana/1b444b75ae9fc0e3d408c8b6dc67cd6949f672876733761724547b6afe798f31" \
  --keypair keypair.json
```

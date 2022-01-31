# Description
A Rust program who reads a CSV listing series of money transactions, handles them by creating client accounts and updating their funds details, and output all account details.
Transactions are processed chronologically matching their appearence order in the CSV.
The details of possible transactions and handled cases are described below.

## CSV format
The CSV must include a header and match the following fields:
`type, client, tx, amount`
Each following line represents a transaction, the transaction type detail can be found in structs::Operation.

See the `resources` directory for some CSV examples.

## Output
The output will be printed to stdout and will be formated as CSV data with the following fields:
`client,available,held,total,locked`

## Invalid transaction handling
Some operations are considered invalid, and will be ignored by the program:
- withdrawal without enough available funds
- chargeback or resolve on a transaction not disputed, or with a dispute already settled/handled
- dispute on an operation different than a deposit
- dispute on a deposit more than once

You can choose to block all transactions as soon as a client account is locked, this behaviour can be turned on with env values read by the program, see Env behaviour section below.

# Usage
This project is a simple Rust crate, use with the usual cargo commands.
```bash
sh$ cargo --version
cargo 1.58.0 (f01b232bc 2022-01-19)
sh$ rustc --version
rustc 1.58.1 (db9d1b20b 2022-01-20)

sh$ cargo run -- resources/example.csv
...
client,available,held,total,locked
1,1.5,0.0,1.5,false
2,2.0,0.0,2.0,false
```

## Env behaviour
The `BLOCK_LOCKED_ACCOUNTS` env variable can be set to change the behaviour for locked accounts.
```bash
sh$ BLOCK_LOCKED_ACCOUNTS=true cargo run -- resources/example_dispute.csv
...
1,21.1,0.0,21.1,true
sh$ BLOCK_LOCKED_ACCOUNTS=false cargo run -- resources/example_dispute.csv
...
client,available,held,total,locked
1,19.1,2.0,21.1,true
```

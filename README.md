# OmniPaxos-SQL

A distributed SQL database system built on [OmniPaxos](https://omnipaxos.com) consensus protocol with SQLite as the underlying storage engine. This project extends the original key-value store functionality to support SQL queries with variable consistency levels.

## Features
- **SQL Support**: Execute SQL queries in a distributed environment
- **Variable Consistency**: Choose between three consistency levels (Leader, Local, Linearizable) for each read operation
- **Performance Tradeoffs**: Balance between consistency and performance based on application needs

## Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Docker](https://www.docker.com/)
- SQLite

## How to run

### Local Development
The project can be run locally using the included configuration files:

```bash
# Run the SQLite consistency tests
cargo run --bin db_test

# Start a three-node cluster (run each in separate terminals)
cargo run --bin server -- --config=./build_scripts/server-1-config.toml
cargo run --bin server -- --config=./build_scripts/server-2-config.toml
cargo run --bin server -- --config=./build_scripts/server-3-config.toml

# Run a client that connects to the cluster
cargo run --bin client -- --config=./client-config.toml

# Run the test client
cargo run --bin test_client
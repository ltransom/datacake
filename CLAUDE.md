# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Datacake is a batteries-included framework for building fault-tolerant distributed data systems in Rust. It implements eventually consistent replication using CRDTs (Conflict-free Replicated Data Types) with hybrid logical clocks, designed as an alternative to Raft consensus for specific use cases.

## Commands

### Building and Testing
- `cargo build` - Build the entire workspace
- `cargo test --workspace --exclude simulation-tests --features test-utils` - Run main test suite
- `cargo test --doc --workspace --exclude simulation-tests --features test-utils` - Run documentation tests
- `cargo nextest run --workspace --exclude simulation-tests --retries 3 --features test-utils` - Run tests with nextest (preferred)
- `cargo nextest run -p simulation-tests --retries 3` - Run simulation tests separately

### Code Quality
- `cargo fmt --all -- --check` - Check code formatting (use nightly toolchain)
- `cargo clippy --workspace --all-features --tests --examples --bins -- -Dclippy::todo` - Run clippy lints
- `cargo hack check --all --ignore-private --each-feature --no-dev-deps` - Check feature combinations

### Development Requirements
- Protobuf compiler (`protoc`) is required for building
- `cargo-hack` tool needed for feature checking: `cargo install cargo-hack`
- `cargo-nextest` tool recommended for testing: `cargo install cargo-nextest`

## Architecture

### Workspace Structure
Datacake is organized as a Cargo workspace with the following key crates:

- **`datacake-crdt`** - Core CRDT implementation with HLCTimestamp (Hybrid Logical Clock)
- **`datacake-node`** - Cluster membership system built on chitchat (scuttlebutt gossip protocol)
- **`datacake-rpc`** - Zero-copy RPC framework using HTTP/2 and rkyv serialization
- **`datacake-eventual-consistency`** - High-level eventually consistent store framework
- **`datacake-sqlite`** - Pre-built SQLite storage implementation
- **`datacake-lmdb`** - Pre-built LMDB storage implementation

### Core Concepts

**Node Architecture**: Datacake uses a node-based architecture where each node runs a membership system, RPC server, and can have multiple extensions attached at runtime.

**Extensions System**: The framework follows an extension pattern where storage backends and replication logic are implemented as `ClusterExtension` traits that can be added to nodes dynamically.

**CRDT-Based Replication**: State synchronization uses ORSwot (Observed-Remove Set without Tombstones) CRDTs with hybrid logical clocks for causality tracking.

**Storage Abstraction**: The `Storage` trait defines the interface for persistent backends, with implementations for SQLite and LMDB provided.

### Key Files

- `src/lib.rs` - Main crate re-exports with feature gates
- `datacake-eventual-consistency/src/storage.rs` - Core storage trait definition
- `datacake-node/src/lib.rs` - Node membership and extension system
- `datacake-crdt/src/orswot.rs` - CRDT implementation
- `datacake-rpc/src/` - RPC framework implementation

## Development Guidelines

### Features and Optional Dependencies
The project uses extensive feature flags. Default features include `datacake-crdt`, `datacake-rpc`, `datacake-eventual-consistency`, and `datacake-node`. Storage backends (SQLite/LMDB) and rkyv serialization support are optional.

### Testing Strategy
- Unit tests for individual components
- Integration tests in `datacake-eventual-consistency/tests/`
- Simulation tests in `simulation-tests/` crate for distributed scenarios
- Test utilities provided in `test-helper/` crate

### Code Style
- Uses rustfmt with custom configuration in `rustfmt.toml`
- Max width: 89 characters
- Nightly rustfmt features enabled
- Clippy with strict linting including `-Dclippy::todo`

## Examples

Check the `examples/` directory for practical implementations:
- `examples/replicated-kv/` - A distributed key-value store built on SQLite
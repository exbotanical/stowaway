#!/usr/bin/env bash
cargo build --release
cp target/release/stowaway /usr/local/bin/stowaway
chmod +x /usr/local/bin/stowaway

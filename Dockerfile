FROM rust:1.89-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y git gcc make curl && rm -rf /var/lib/apt/lists/*

RUN sh -c "`curl -L https://raw.githubusercontent.com/rylnd/shpec/master/install.sh`"

COPY --from=builder /app/target/release/stowaway /usr/local/bin/stowaway
RUN chmod +x /usr/local/bin/stowaway

COPY tests/ /tests/
COPY scripts/ /scripts/
RUN chmod +x /scripts/*.bash /scripts/*.sh

WORKDIR /

ENTRYPOINT ["/entrypoint.sh"]

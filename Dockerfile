# ---- Stage 1: Build ----
FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies: copy manifests first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy project files
COPY diesel.toml ./
COPY migrations ./migrations
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ---- Stage 2: Runtime ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/users-microservice .

EXPOSE 8080
CMD ["./users-microservice"]

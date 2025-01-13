# Frontend Build
FROM node:20 AS frontend
WORKDIR /app/frontend
COPY frontend .
RUN yarn install
RUN yarn build

# Backend Build
FROM rust:latest AS builder
WORKDIR /app/backend
COPY backend .
RUN cargo build --release

# Final Image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libsqlite3-dev libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/backend/target/release/aiswitch /app/
COPY --from=frontend /app/backend/static /app/static
CMD ["./aiswitch"]
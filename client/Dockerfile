# Build context = repo root (see docker-compose.yml).
# Stage 1: Trunk wasm; stage 2: nginx + dist + deploy/nginx.conf.
FROM rust:1.88-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk --locked --version 0.21.14

WORKDIR /src
COPY client/ /src/
WORKDIR /src/crates/app
RUN trunk build --release

FROM nginx:1.27-alpine

COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /src/crates/app/dist/ /usr/share/nginx/html/

EXPOSE 80

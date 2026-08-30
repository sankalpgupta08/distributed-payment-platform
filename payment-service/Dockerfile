FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN useradd --create-home --uid 10001 appuser
COPY --from=builder /app/target/release/payment-service /usr/local/bin/payment-service

USER appuser
EXPOSE 3000
CMD ["payment-service"]

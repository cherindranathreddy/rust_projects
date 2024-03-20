# build stage
FROM rust:1.69-buster as builder

WORKDIR /app
COPY . .

ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL

RUN cargo build --release

# production stage
FROM debian:buster-slim

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/rust-psql-crud-api .

CMD ["./rust-psql-crud-api"]

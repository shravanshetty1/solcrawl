FROM rust:1.60

WORKDIR /usr/src/solcrawler
COPY . .
RUN cargo build
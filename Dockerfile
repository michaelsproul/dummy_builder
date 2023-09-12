FROM rust:latest AS builder
RUN apt-get update && apt-get -y upgrade && apt-get install -y cmake libclang-dev protobuf-compiler
COPY . dummy_builder
RUN cd dummy_builder && cargo install --path . --force --locked

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /usr/local/cargo/bin/dummy_builder /usr/local/bin/dummy_builder
ENTRYPOINT ["dummy_builder"]

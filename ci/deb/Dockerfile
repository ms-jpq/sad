FROM rust:latest


# SET UP
WORKDIR /WORK
RUN cargo install cargo-deb


# COPY
COPY . /WORK


# BUILD
RUN cargo build --verbose
RUN cargo test --verbose


FROM rust:latest


# SET UP
WORKDIR /WORK
RUN cargo install cargo-deb


# COPY
COPY . .


# BUILD
RUN cargo test


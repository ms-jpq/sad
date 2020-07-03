FROM rust:latest


# SET UP
WORKDIR /WORK
RUN cargo install cargo-deb


# COPY
COPY . .


# BUILD
RUN cargo test
RUN cargo deb --target=x86_64-unknown-linux-gnu \
              --output=target/debian/debian.deb

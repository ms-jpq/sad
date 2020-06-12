FROM rust:latest


# SET UP
WORKDIR /WORK
RUN mkdir -p artifacts && \
    apt update && \
    apt install -y zip


# COPY
COPY . .


# BUILD
RUN cargo test
RUN cargo build --release --target=x86_64-unknown-linux-gnu
RUN zip -j artifacts/sad.zip target/release/sad


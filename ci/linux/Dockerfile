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
RUN cargo build --release
RUN zip -j artifacts/sad.zip target/release/sad


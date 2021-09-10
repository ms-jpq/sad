FROM rust:latest


# SET UP
WORKDIR /WORK
RUN mkdir --parents -- artifacts && \
    apt update && \
    apt install --yes -- zip && \
    rustup target add x86_64-pc-windows-gnu


# COPY
COPY . .


# BUILD
RUN cargo test
RUN cargo build --release --locked --target=x86_64-unknown-linux-gnu
RUN zip -j artifacts/sad.zip target/x86_64-unknown-linux-gnu/release/sad

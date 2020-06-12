FROM rust:latest


# SET UP
WORKDIR /WORK
RUN apt update && \
    apt install -y zip


# COPY
COPY . .


# BUILD
RUN cargo test
RUN cargo build --release
RUN zip -j sad.zip target/release/sad


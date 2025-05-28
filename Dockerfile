FROM rust:1.87.0 as builder

RUN apt-get update && apt-get install -y musl-tools

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN cargo fetch

COPY . .

RUN cargo build --target x86_64-unknown-linux-musl --release --bin dict_combine 

RUN ./target/x86_64-unknown-linux-musl/release/dict_combine ./content/ --overwrite

RUN cargo install --target x86_64-unknown-linux-musl --path . --bin kate_bot

FROM scratch

COPY --from=builder /usr/local/cargo/bin/kate_bot .

CMD [ "./kate_bot" ]

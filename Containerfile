FROM docker.io/library/rust:1.72 as build

RUN rustup target add x86_64-unknown-linux-musl

RUN apt-get update
RUN apt-get install musl-tools -y

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs

RUN cargo build --release --target x86_64-unknown-linux-musl

COPY . .

# now rebuild with the proper main
RUN touch src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl

### RUN
FROM gcr.io/distroless/static-debian12

WORKDIR /app

COPY --from=build /app/target/release/x86_64-unknown-linux-musl/uptime uptime

CMD ["/app/uptime"]

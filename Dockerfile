FROM docker.io/library/rust:1.72 as build

RUN apt-get update
RUN apt-get install musl-tools -y

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs

RUN cargo build --release

COPY . .

# now rebuild with the proper main
RUN touch src/main.rs
RUN cargo build --release

### RUN
FROM gcr.io/distroless/cc

WORKDIR /app

COPY --from=build /app/target/release/x86_64-unknown-linux-musl/uptime uptime

CMD ["/app/uptime"]

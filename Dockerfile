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
FROM docker.io/library/debian:trixie-20230814-slim

WORKDIR /app

COPY --from=build /app/target/release/uptime uptime

CMD ["/app/uptime"]

FROM rust as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release --bin farmhand

FROM rust as runtime
WORKDIR /app
COPY --from=builder /app/target/release/farmhand /usr/local/bin
EXPOSE 3000
CMD ["/usr/local/bin/farmhand"]

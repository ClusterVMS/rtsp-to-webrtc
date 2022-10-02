# Base image for the cargo-chef compilation steps
# In order to target alpine as the runtime, which uses musl libc, we need to use muslrust
# We also need to use the nightly version of Rust because Rocket requires it
FROM clux/muslrust:1.66.0-nightly-2022-09-24 as chef
RUN cargo install cargo-chef@0.1.44 --locked
WORKDIR /app



# Planner that just gathers the list of dependencies
FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json



# Builder that actually compiles dependencies and then our application
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin rtsp-to-webrtc



# Container to run the application
FROM alpine:3.16 as runtime
RUN addgroup -S clustervms-user && adduser -S clustervms-user -G clustervms-user
USER clustervms-user
ENV ROCKET_ADDRESS="0.0.0.0"
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rtsp-to-webrtc /app/rtsp-to-webrtc
COPY ./Rocket.toml /app/
ENTRYPOINT ["/app/rtsp-to-webrtc"]

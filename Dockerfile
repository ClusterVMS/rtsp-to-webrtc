# Base image for the cargo-chef compilation steps
FROM cgr.dev/chainguard/rust:latest as chef
RUN cargo install cargo-chef@0.1.61 --locked
WORKDIR /app



# Planner that just gathers the list of dependencies
FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json



# Builder that actually compiles dependencies and then our application
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin rtsp-to-webrtc



# Container to run the application
FROM cgr.dev/chainguard/glibc-dynamic:latest as runtime
# Chainguard image is already set up to run as non-root
ENV ROCKET_ADDRESS="0.0.0.0"
WORKDIR /app
COPY --from=builder /app/target/release/rtsp-to-webrtc /app/rtsp-to-webrtc
COPY ./Rocket.toml /app/
ENTRYPOINT ["/app/rtsp-to-webrtc"]

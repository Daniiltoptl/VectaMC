FROM rustlang/rust:nightly-alpine3.23 AS builder
LABEL authors="junkydeveloper"

WORKDIR /vecta

COPY . .
RUN cargo build --release --locked -p vecta --features stand-alone

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --chmod=755 --from=builder /vecta/target/release/vecta /
LABEL org.opencontainers.image.title="VectaMC"
LABEL org.opencontainers.image.source="https://github.com/Daniiltoptl/VectaMC"

EXPOSE 25565

ENTRYPOINT ["/vecta"]

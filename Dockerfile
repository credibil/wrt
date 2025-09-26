# Dockerfile for building runtime examples

FROM rust:alpine AS build
ARG BIN

RUN apk add --no-cache clang lld musl-dev git perl make ca-certificates
RUN adduser --disabled-password --gecos "" --home "/nonexistent" \
    --shell "/sbin/nologin" --no-create-home --uid 10001 appuser

WORKDIR /app
RUN \
    --mount=type=secret,id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
    --mount=type=bind,src=.cargo,dst=.cargo \
    --mount=type=bind,src=Cargo.toml,dst=Cargo.toml \
    --mount=type=bind,src=Cargo.lock,dst=Cargo.lock \
    --mount=type=bind,src=src,dst=src \
    --mount=type=bind,source=crates,target=crates \
    --mount=type=bind,source=examples,target=examples \
    --mount=type=bind,source=wit,target=wit \
    # --mount=type=cache,target=$CARGO_HOME/git/db \
    # --mount=type=cache,target=$CARGO_HOME/registry \
    cargo build --bin $BIN --release

# N.B. 'alpine' image is ~10Mb larger but appears to perform 
# marginally better than 'scratch'
FROM alpine:latest
# FROM scratch
ARG BIN

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group
COPY --from=build --chown=appuser:appuser /app/target/release/$BIN /bin/server

USER appuser:appuser
EXPOSE 8080
ENTRYPOINT ["/bin/server", "run"]
CMD ["/app.wasm"]
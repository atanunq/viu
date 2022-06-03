FROM rust:slim-buster as build
ARG ARCH
WORKDIR opt
RUN rustup target add $ARCH-unknown-linux-musl
COPY . /opt
RUN cargo build --target $ARCH-unknown-linux-musl --release

FROM alpine:3.15.0
ARG ARCH
COPY --from=build /opt/target/$ARCH-unknown-linux-musl/release/viu /usr/bin
ENTRYPOINT ["viu"]
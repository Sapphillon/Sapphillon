FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Include mold linker to improve build times either by using environment variable
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang busybox mold protobuf-compiler \
   && apt-get autoremove -y && apt-get clean -y

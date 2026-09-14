# syntax=docker/dockerfile:1.7
ARG BASE_IMAGE=ghcr.io/cpeter1207/rpt-advanced-quality-debian13:latest
FROM ${BASE_IMAGE}

# Link the local quality image recipe to its intended source repository.
LABEL org.opencontainers.image.source="https://github.com/cpeter1207/rptadv-rnnoise-adapter"

ARG TARGETARCH
ARG RNNOISE_VERSION=0.2
ARG RNNOISE_SHA256=90fce4b00b9ff24c08dbfe31b82ffd43bae383d85c5535676d28b0a2b11c0d37
ARG RUSTUP_INIT_VERSION=1.28.2
ARG RUST_STABLE=1.85.0
ARG RUST_NIGHTLY=nightly-2025-02-20
ARG CARGO_LLVM_COV_VERSION=0.6.21

ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:${PATH}

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		autoconf automake autopkgtest build-essential ca-certificates cargo clang-tidy \
		cppcheck curl debhelper docker-cli dpkg-dev doxygen graphviz libtool make \
		patch pkg-config rustc shellcheck && \
	rm -rf /var/lib/apt/lists/*

COPY rnnoise/debian/ /tmp/rnnoise-debian/
RUN set -eu; \
	work=$(mktemp -d); \
	trap 'rm -rf "$work" /tmp/rnnoise-debian' EXIT; \
	archive="$work/rnnoise-${RNNOISE_VERSION}.tar.gz"; \
	curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
		"https://github.com/xiph/rnnoise/releases/download/v${RNNOISE_VERSION}/rnnoise-${RNNOISE_VERSION}.tar.gz" \
		--output "$archive"; \
	echo "${RNNOISE_SHA256}  $archive" | sha256sum --check --status; \
	source_dir="$work/rnnoise-${RNNOISE_VERSION}"; \
	mkdir -p "$source_dir"; \
	tar -C "$source_dir" --strip-components=1 -xzf "$archive"; \
	cp -a /tmp/rnnoise-debian "$source_dir/debian"; \
	find "$source_dir/debian" -type f -exec chmod 0644 {} +; \
	chmod 0755 "$source_dir/debian/rules"; \
	cd "$source_dir"; \
	dpkg-buildpackage -us -uc -b; \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		"$work"/librnnoise0_*.deb "$work"/librnnoise-dev_*.deb; \
	ldconfig; \
	pkg-config --exact-version="${RNNOISE_VERSION}" rnnoise

RUN case "${TARGETARCH}" in \
		amd64) rustup_host=x86_64-unknown-linux-gnu; rustup_sha=20a06e644b0d9bd2fbdbfd52d42540bdde820ea7df86e92e533c073da0cdd43c ;; \
		arm64) rustup_host=aarch64-unknown-linux-gnu; rustup_sha=e3853c5a252fca15252d07cb23a1bdd9377a8c6f3efa01531109281ae47f841c ;; \
		*) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
	esac; \
	curl --proto '=https' --tlsv1.2 --fail --silent --show-error \
		"https://static.rust-lang.org/rustup/archive/${RUSTUP_INIT_VERSION}/${rustup_host}/rustup-init" \
		--output /tmp/rustup-init; \
	echo "${rustup_sha}  /tmp/rustup-init" | sha256sum --check --status; \
	chmod 0755 /tmp/rustup-init; \
	/tmp/rustup-init -y --no-modify-path --default-toolchain none; \
	rm -f /tmp/rustup-init

RUN rustup toolchain install "${RUST_STABLE}" --profile minimal --component clippy --component rustfmt && \
	rustup toolchain install "${RUST_NIGHTLY}" --profile minimal --component llvm-tools-preview && \
	rustup default "${RUST_STABLE}" && \
	cargo +"${RUST_NIGHTLY}" install cargo-llvm-cov --version "${CARGO_LLVM_COV_VERSION}" --locked && \
	cargo +"${RUST_NIGHTLY}" llvm-cov --version

# The shared quality base temporarily carries an unmanaged RNNoise bootstrap.
# Remove only those exact files so link and shlibs checks exercise the packages
# built above rather than resolving an unowned /usr/local installation.
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		iproute2 && \
	rm -rf /var/lib/apt/lists/* && \
	rm -f /usr/local/include/rnnoise.h /usr/local/lib/librnnoise.a \
		/usr/local/lib/librnnoise.la /usr/local/lib/librnnoise.so \
		/usr/local/lib/librnnoise.so.0 /usr/local/lib/librnnoise.so.0.4.1 \
		/usr/local/lib/pkgconfig/rnnoise.pc && \
	ldconfig && \
	test "$(pkg-config --variable=prefix rnnoise)" = /usr

WORKDIR /workspace

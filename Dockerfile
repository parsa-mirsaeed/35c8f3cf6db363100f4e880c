# syntax=docker/dockerfile:1.7
ARG RUST_IMAGE=rust:1.96.1-trixie
FROM ${RUST_IMAGE} AS toolchain

ARG DIOXUS_CLI_VERSION=0.7.2
ARG CARGO_CHEF_VERSION=0.1.77
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown \
    && curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall "cargo-chef@${CARGO_CHEF_VERSION}" -y --force \
    && cargo binstall "dioxus-cli@${DIOXUS_CLI_VERSION}" -y --force

WORKDIR /workspace

FROM toolchain AS dev
CMD ["dx", "serve", "--web", "--package", "web", "--addr", "0.0.0.0", "--port", "8080"]

FROM toolchain AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM toolchain AS builder
RUN apt-get update \
    && apt-get install --yes --no-install-recommends postgresql postgresql-client \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /workspace/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package web --features server
RUN cargo chef cook --release --recipe-path recipe.json --package web --features web --target wasm32-unknown-unknown
COPY . .
RUN set -eux; \
    service postgresql start; \
    runuser -u postgres -- psql --set=ON_ERROR_STOP=1 \
        --command="ALTER USER postgres PASSWORD 'postgres'"; \
    runuser -u postgres -- createdb edutalent_build; \
    export DATABASE_URL='postgresql://postgres:postgres@127.0.0.1:5432/edutalent_build'; \
    bash scripts/ci/apply_migrations.sh; \
    cargo build --release --package api --features server --bin ai_gateway; \
    dx bundle --web --release --package web; \
    service postgresql stop
RUN set -eux; \
    bundle_dir="target/dx/web/release/web"; \
    test -d "${bundle_dir}/public"; \
    mkdir -p /opt/edutalent-bundle; \
    cp -R "${bundle_dir}/public" /opt/edutalent-bundle/public; \
    if [ -x "${bundle_dir}/server" ]; then \
        cp "${bundle_dir}/server" /opt/edutalent-bundle/server; \
    elif [ -x "${bundle_dir}/web" ]; then \
        cp "${bundle_dir}/web" /opt/edutalent-bundle/server; \
    else \
        executable="$(find "${bundle_dir}" -maxdepth 1 -type f -perm /111 | head -n 1)"; \
        test -n "${executable}"; \
        cp "${executable}" /opt/edutalent-bundle/server; \
    fi; \
    test -x target/release/ai_gateway; \
    cp target/release/ai_gateway /opt/edutalent-bundle/ai_gateway; \
    if [ -d packages/web/assets/fonts ]; then \
        mkdir -p /opt/edutalent-bundle/public/fonts; \
        cp -R packages/web/assets/fonts/. /opt/edutalent-bundle/public/fonts/; \
    fi; \
    chmod +x /opt/edutalent-bundle/server /opt/edutalent-bundle/ai_gateway

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/edutalent
COPY --from=builder /opt/edutalent-bundle/ /opt/edutalent/
COPY packages/api/migration/migrations/ /opt/edutalent/packages/api/migration/migrations/
COPY migrations/ /opt/edutalent/migrations/
COPY scripts/ci/apply_migrations.sh /opt/edutalent/scripts/ci/apply_migrations.sh
COPY scripts/ci/configure_database_role.sh /opt/edutalent/scripts/ci/configure_database_role.sh
COPY docker/entrypoint.sh /usr/local/bin/edutalent-entrypoint
RUN chmod +x \
    /usr/local/bin/edutalent-entrypoint \
    /opt/edutalent/server \
    /opt/edutalent/ai_gateway \
    /opt/edutalent/scripts/ci/apply_migrations.sh \
    /opt/edutalent/scripts/ci/configure_database_role.sh

ENV IP=0.0.0.0 \
    PORT=8080 \
    RUN_MIGRATIONS=true

EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=30s --retries=6 \
    CMD curl --fail --silent http://127.0.0.1:${PORT}/healthz >/dev/null || exit 1

ENTRYPOINT ["edutalent-entrypoint"]
CMD ["server"]

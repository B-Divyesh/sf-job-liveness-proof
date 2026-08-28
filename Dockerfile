ARG BUILD_SHA=development

FROM node:22-alpine AS frontend
WORKDIR /build
COPY package.json package-lock.json tsconfig.json vite.config.ts ./
COPY frontend ./frontend
RUN npm ci && npm run build

FROM rust:1.88-bookworm AS backend
ARG BUILD_SHA
ENV BUILD_SHA=${BUILD_SHA}
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --locked --release --bin run-proof-server --bin run-proof

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 runproof && useradd --uid 10001 --gid runproof --create-home runproof \
    && mkdir -p /app/dist /data && chown -R runproof:runproof /app /data
WORKDIR /app
COPY --from=backend /build/target/release/run-proof-server /usr/local/bin/run-proof-server
COPY --from=backend /build/target/release/run-proof /usr/local/bin/run-proof
COPY --from=frontend /build/dist ./dist
USER 10001:10001
EXPOSE 8080
VOLUME ["/data"]
ENTRYPOINT ["run-proof-server"]

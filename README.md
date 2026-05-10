# fscl-outbox-publisher

Publisher sidecar/binary for relaying outbox rows from a bounded-context database to NATS.

It currently contains:

- environment-driven runtime config
- PostgreSQL LISTEN setup on the shared outbox notify channel
- shared outbox row mapping into `fscl_messaging::OutboxRecord`
- startup-time outbox schema installation via `fscl_messaging::ensure_outbox_schema`
- publisher-local settings such as batch size and fallback polling interval

## Dependencies

```text
fscl-outbox-publisher
    |
    +--> fscl-messaging
    |
    +--> fscl-core

fscl-messaging -> event/outbox contract
fscl-core      -> shared core types used by transitional code paths
```


## Split

```text
fscl-messaging contract     -> OUTBOX_NOTIFY_CHANNEL, schema, envelope types
process environment         -> runtime values from shell or container
```

The outbox table/trigger/function SQL is applied from `fscl-messaging` during startup, so the publisher does not need a local copy of the schema file.

## Config

Current runtime config is parsed in `src/config.rs`.

Runtime values are read from process environment variables (shell or container).

Config points:

- `DB_HOST`: PostgreSQL host for the bounded-context (view-specific) database
- `DB_PORT`: PostgreSQL port
- `DB_USER`: PostgreSQL user
- `DB_PASSWORD`: PostgreSQL password
- `DB_NAME`: PostgreSQL database name
- `NATS_URL`: NATS connection URL used for publishing
- `OUTBOX_SUBJECT_PREFIX`: subject namespace prefix, for example `fscl.process`
- `OUTBOX_BATCH_SIZE`: max number of rows processed per batch when polling/draining is implemented
- `OUTBOX_FALLBACK_POLL_MS`: polling interval used when LISTEN notifications are not sufficient

## Dev Setup

Load local dev secrets from the compose helper:

```sh
source ../compose/load-secrets.sh
```

Run directly:

```sh
cd fscl-outbox-publisher
cargo test
cargo run
```

Or run with the shared Compose scaffolding:

```sh
source ../compose/load-secrets.sh
docker compose -p fscl -f ../compose/process-stack.yaml up
```

## K8s Setup

Scaffold only for now.

The Kubernetes wiring lives under [fscl/doc/rust/fscl-k8s](../fscl/doc/rust/fscl-k8s) and currently assumes the publisher runs as a sidecar next to `process-api`.

Relevant scaffold pieces:

- `15-process-messaging.yaml`: shared messaging runtime values for the bounded context
- `20-process-api.yaml`: sidecar container wiring
- `22-outbox-publisher.yaml`: publisher-local runtime values

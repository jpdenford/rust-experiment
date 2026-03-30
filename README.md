# Sensor experiment

Rough plan with intention to

- refresh & broaden my rust knowledge through exploration.
- prove my engineering experience can be applied to new technologies incl. my beginner/intermediate level rust.

## Getting started

```bash
./setup-dev.sh     # creates admin-token.json and ~/.influxdb3/data
docker compose up  # starts InfluxDB and initialises the database + table
```

InfluxDB dev token: `apiv3_dev-local-token`

Rouch Client + Server model

- Client-1 (esp-32, rust): gather sensor data and send it over the wire (usb/uart?) to be ingested by ingestion process.
- Client-2: (web/nextjs/react). Display aggregated sensor data in a web view.
  - 1. button to rest query data in influxdb
  - 2. (stretch) shows hosts perspective react component connecting to perspective rust server.
- DB (influxdb): persist and compute for sensor data.
- Server (rust):
  - Entrypoint-1: standalone process to persist data to db.
    - 1. persist data to influxdb
    - In future: also handle init/migrations etc.
  - Entrypoint-2: Axum w. Perspective
    - 1. Axum web server which serves an aggregated view of the data
    - 2. stretch: 'Perspective' view for dynamic queries

## Guiding principles

- Safe numeric conversions
- defensive programming (assert assumptions)
- LLM use - avoid Agentic coding in rust. Only use it like google to ask questions if/when stuck. Need the compiler to push back to internalise the info!

MVP / impl plan:

1.  Rust server which persists to db.

## Lessons

- InfluxDB WAL flush has a default of 1s causing each write to take 1s!
  - 1. reduce as per docs for local disks (or mark writes not requiring wal write)
  - 2. bigger batches & multiple concurrent writes in-flight
- use '\_' instead of '-' in module names
- `fn to_x` is the convention for **borrowed** self `&self`, `fn into_x` **consumes** self
- need to 'pin' tokio stream in order to run/consume from it

## Setup

```sh
./setup-dev.sh
docker compose up -d
# run the server
cargo run --manifest-path crates/server/Cargo.toml
```

### Agentic code Registry

- init of influxdb for local dev. Reason: not main focus / plumbing

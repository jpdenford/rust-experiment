# Sensor experiment

Rough plan with intention to

- refresh & broaden my rust knowledge through exploration.
- prove my engineering experience can be applied to new technologies incl. my beginner/intermediate level rust.

## Plan Sketch

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

MVP / impl plan:

1.  Rust server which persists to db.

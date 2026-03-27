# trainwreck

![trainwreck logo](trainwreck_logo.png)

A CLI tool for displaying transit departure schedules from [GTFS](https://gtfs.org/) archives, with optional real-time delay and cancellation data from the [Swiss OpenTransportData](https://opentransportdata.swiss/) API.

## Features

- Browse stops and departures from any GTFS static feed (ZIP archive)
- Case-insensitive stop search by name
- Real-time departure status: delays, cancellations, and skipped stops via GTFS-RT
- Graceful fallback to static schedule when real-time data is unavailable

## Installation

Requires Rust (2024 edition). Clone and build with:

```sh
git clone https://github.com/killerfoxi/trainwreck
cd trainwreck
cargo build --release
```

The binary will be at `target/release/trainwreck`.

## Usage

```
trainwreck <GTFS_ZIP> [STOP_QUERY] [--api-key <KEY>]
```

### List all stops

```sh
trainwreck feed.zip
```

### Show departures for a stop

```sh
trainwreck feed.zip "Zürich HB"
```

Displays a summary of routes serving the stop, including destination, transit type, trip count, and first departure.

### Show real-time departures

```sh
trainwreck feed.zip "Zürich HB" --api-key <your-otd-api-key>
# or via environment variable
OTD_API_KEY=<key> trainwreck feed.zip "Zürich HB"
```

Each departure is annotated with its live status:

| Annotation | Meaning |
|---|---|
| `[on time]` | No delay |
| `[+2m 30s]` | Running late |
| `[-1m 0s]` | Running early |
| `[CANCELLED]` | Trip cancelled |
| `[STOP SKIPPED]` | Stop not served on this run |

## Getting GTFS data

GTFS feeds are published by transit agencies worldwide. For Switzerland, feeds are available from [opentransportdata.swiss](https://opentransportdata.swiss/). An API key from the same platform unlocks the real-time GTFS-RT endpoint.

## Tech stack

- **Rust** — async via Tokio, error handling via `thiserror` + `color-eyre`
- **GTFS static** — ZIP + CSV parsing with streaming/filtered reads
- **GTFS-RT** — protobuf decoding via `prost`, HTTP via `reqwest`
- **CLI** — `clap` with `--api-key` flag and `OTD_API_KEY` env var support

## License

MIT

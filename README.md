# trainwreck

![trainwreck logo](trainwreck_logo.png)

A transit departure schedule viewer built on [GTFS](https://gtfs.org/) archives, with optional real-time delay and cancellation data from the [Swiss OpenTransportData](https://opentransportdata.swiss/) API. Run it as a command-line tool or spin up a local web server with a built-in departure board.

## Features

- Browse stops and departures from any GTFS static feed (ZIP archive)
- Case-insensitive stop search by name
- Real-time departure status: delays, cancellations, and skipped stops via GTFS-RT
- Graceful fallback to static schedule when real-time data is unavailable
- REST API for programmatic access to stops and schedules
- Web UI with live search, departure board, and per-stop refresh

## Installation

Requires Rust (2024 edition). Clone and build with:

```sh
git clone https://github.com/killerfoxi/trainwreck
cd trainwreck
cargo build --release
```

The binary will be at `target/release/trainwreck`.

To build with the embedded web UI included in the binary:

```sh
cargo build --release --features embed-web
```

## Usage

trainwreck has two modes: `query` for terminal output and `web` for the HTTP server.

### Query mode

```
trainwreck query <GTFS_ZIP> [STOP_QUERY] [--api-key <KEY>]
```

#### List all stops

```sh
trainwreck query feed.zip
```

#### Show departures for a stop

```sh
trainwreck query feed.zip "Zürich HB"
```

Displays routes serving the stop, including destination, transit type, trip count, and first departure.

#### Show real-time departures

```sh
trainwreck query feed.zip "Zürich HB" --api-key <your-otd-api-key>
# or via environment variable
OTD_API_KEY=<key> trainwreck query feed.zip "Zürich HB"
```

Each departure is annotated with its live status:

| Annotation | Meaning |
|---|---|
| `[on time]` | No delay |
| `[+2m 30s]` | Running late |
| `[-1m 0s]` | Running early |
| `[CANCELLED]` | Trip cancelled |
| `[STOP SKIPPED]` | Stop not served on this run |

### Web mode

```
trainwreck web <GTFS_ZIP> [--api-key <KEY>] [--bind <ADDR>]
```

Starts an HTTP server (default: `127.0.0.1:3000`) serving both a REST API and the web departure board.

```sh
trainwreck web feed.zip --api-key <your-otd-api-key>
```

Open `http://localhost:3000` in a browser to use the departure board, or call the API endpoints directly.

## REST API

### Search stops

```
GET /api/stops?q=<query>
```

Returns stops whose names match the query (case-insensitive substring). Omit `q` to return all stops. Only top-level stations are returned, not individual platforms.

```json
[
  { "stop_id": "8503000", "stop_name": "Zürich HB" }
]
```

### Get departures

```
GET /api/schedule?stop_ids=<id1,id2,...>
```

Returns upcoming departures for the given stop IDs. Station families are expanded automatically so querying any platform or parent stop returns the full picture. If the server was started with an API key, results include real-time status.

```json
[
  {
    "trip_id": "...",
    "stop_id": "8503000",
    "departure_secs": 52200,
    "route_name": "IC 1",
    "route_type": 2,
    "destination": "Genève-Aéroport",
    "delay_secs": 120,
    "canceled": false,
    "skipped": false,
    "platform": "7"
  }
]
```

## Web UI

The departure board runs in the browser as a WebAssembly app. It provides:

- **Stop search** — live, debounced search as you type; click any result to load departures
- **Departure board** — shows arrivals from 1 minute ago up to 3 hours ahead, with formatted times, relative countdowns, and transit type badges
- **Real-time status badges** — on time, delayed (with offset), early, cancelled, or skipped
- **Platform display** — track/platform number when available in the feed
- **Refresh button** — re-fetch current departures on demand
- **API key input** — enter your OTD API key directly in the UI to enable real-time updates (stored in browser localStorage)

Route type badges are color-coded: rail (blue), tram (red), bus (orange), subway (purple), ferry (teal).

## Getting GTFS data

GTFS feeds are published by transit agencies worldwide. For Switzerland, feeds are available from [opentransportdata.swiss](https://opentransportdata.swiss/). An API key from the same platform unlocks the real-time GTFS-RT endpoint.

## Tech stack

- **Rust** — async via Tokio, error handling via `thiserror` + `color-eyre`
- **GTFS static** — ZIP + CSV parsing with streaming/filtered reads
- **GTFS-RT** — protobuf decoding via `prost`, HTTP via `reqwest`
- **CLI** — `clap` with `--api-key` flag and `OTD_API_KEY` env var support
- **Web server** — `axum` with JSON API and static file serving via `tower-http`
- **Web UI** — Leptos 0.8 (Rust → WASM), compiled with `trunk`

## License

MIT

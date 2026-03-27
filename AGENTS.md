# AGENTS.md — trainwreck project context

## Environment

This project uses **devenv** (Nix-based) for its toolchain. The shell environment is
activated by **direnv** when you `cd` into the project directory. Claude Code's Bash
tool does NOT inherit this automatically.

**Always prefix Bash commands with the direnv export trick:**

```sh
eval "$(direnv export bash 2>/dev/null)" && cargo <subcommand>
```

Or ask the user to run commands inside an active `devenv shell`.

The Rust toolchain (cargo, rustc, clippy, etc.) lives at:
`.devenv/profile/bin/` — but using the direnv export is cleaner than hardcoding paths.

---

## Project overview

**trainwreck** is a CLI that reads a GTFS static zip archive and displays departure
schedules for a given stop. When an API key is provided it enriches each departure with
real-time delay/cancellation data from the OpenTransportData Swiss GTFS-RT feed.

```
trainwreck <path-to-gtfs.zip> [stop-query] [--api-key <key>]
OTD_API_KEY=<key> trainwreck feed.zip "Zürich HB"
```

- No stop query → lists all stops.
- Stop query, no API key → static route summary per route.
- Stop query + API key → per-departure list with `[+Xm Ys]` / `[on time]` / `[CANCELLED]` / `[STOP SKIPPED]` annotations.

---

## Code conventions

- **Edition:** 2024
- **`#![deny(clippy::pedantic)]`** at the crate root (`src/main.rs`). All code must pass
  `cargo clippy -- -D clippy::pedantic` with zero warnings.
- **No `mod.rs`** — use the modern Rust module layout: `src/foo.rs` as the module file
  with submodules in `src/foo/`.
- **Error handling:** `thiserror` for library-style errors, `color-eyre` for top-level
  CLI reporting. No `.unwrap()`.
- **CLI args:** `clap` with the `derive` feature.
- **Default features disabled** for all dependencies; only explicitly required features
  are enabled.

---

## Module layout

```
src/
  main.rs              — #[tokio::main], clap Args, show_schedule / print_departures
  gtfs.rs              — re-exports GtfsArchive, StopSchedule
  gtfs/
    archive.rs         — opens the zip, reads CSV files
    error.rs           — GtfsError (thiserror)
    model.rs           — Stop, StopTime, Trip, Route, RouteType (serde Deserialize)
    query.rs           — StopSchedule, RouteSummary, departures()
  realtime.rs          — re-exports fetch_trip_updates, DepartureStatus, RealtimeFeed
  realtime/
    proto.rs           — include!(transit_realtime.rs) from OUT_DIR
    error.rs           — RealtimeError (reqwest + prost::DecodeError)
    model.rs           — RealtimeFeed, TripStatus, StopTimeStatus, DepartureStatus
    client.rs          — fetch_trip_updates() — HTTP + protobuf decode
build.rs               — prost_build::compile_protos(["proto/gtfs-realtime.proto"])
proto/
  gtfs-realtime.proto  — official Google Transit proto (proto2, package transit_realtime)
```

---

## Key dependency notes

| Crate | Features enabled | Why |
|---|---|---|
| `clap` | `derive, env, std` | `std` is required (not default) |
| `prost` | `derive, std` | `derive` enables the `Message`/`Enumeration` proc-macros used by codegen |
| `reqwest` | `rustls` | In v0.13 the feature was renamed from `rustls-tls` to `rustls`; enabling it also auto-enables `__rustls-aws-lc-rs` |
| `tokio` | `macros, rt-multi-thread` | `macros` for `#[tokio::main]` |

---

## Proto codegen

`build.rs` compiles `proto/gtfs-realtime.proto` via `prost_build`. The generated file
lands at `target/debug/build/trainwreck-<hash>/out/transit_realtime.rs` and is
`include!`d by `src/realtime/proto.rs`.

Relevant generated type paths (confirmed against the actual output):

- `FeedMessage` — top-level, field `entity: Vec<FeedEntity>`
- `FeedEntity` — field `trip_update: Option<TripUpdate>`
- `TripUpdate` — field `trip: TripDescriptor` (**not** `Option` — it is `required` in proto2), `stop_time_update: Vec<trip_update::StopTimeUpdate>`
- `trip_update::StopTimeUpdate` — fields `stop_id`, `arrival`, `departure` (both `Option<StopTimeEvent>`), `schedule_relationship: Option<i32>`
- `TripDescriptor` — fields `trip_id: Option<String>`, `schedule_relationship: Option<i32>`
- Cancellation discriminant: `TripDescriptor::ScheduleRelationship::CANCELED = 3`
- Skip discriminant: `StopTimeUpdate::ScheduleRelationship::SKIPPED = 1`

---

## Real-time feed endpoint

```
GET https://api.opentransportdata.swiss/la/gtfs-rt
Authorization: Bearer <api_key>
```

Response is a binary protobuf `FeedMessage`. A bad key returns an HTTP error, which
propagates as `RealtimeError::Http` and prints `Warning: real-time data unavailable: …`
before falling back to static display.

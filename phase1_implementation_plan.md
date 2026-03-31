# Halestorm Phase 1 — Implementation Plan

## Context

Halestorm is a 2D tile-based online RPG in Rust/Bevy. No code exists yet — only a design document. Phase 1 delivers the core engine: a player can create an account, create a character, spawn into a tile map, and walk around with server-authoritative movement. Both single-player (embedded server) and multiplayer (quinn networking) modes work.

**Milestone definition:** A playable experience where you can launch the game, create an account/character, spawn on a map, and walk around. Not just technical scaffolding — something that starts and works.

---

## Version Targets

- **Bevy 0.18** (latest stable as of March 2026)
- **bevy_ecs_tilemap 0.18.1** (Bevy 0.18 compatible)
- **bevy_ecs_tiled 0.11** (for loading Tiled .tmj maps, Bevy 0.18 compatible)
- **quinn 0.11.x** (QUIC networking)
- **serde + bincode** (serialization)
- **tokio** (async runtime for quinn)
- **rcgen** (self-signed TLS certs for dev)

---

## Architecture: Transport Abstraction

The key architectural decision enabling single-player and multiplayer with shared game logic:

```
Client Input → ClientMessage → [Transport] → Server MessageInbox
Server Simulation → ServerMessage → [Transport] → Client MessageInbox
```

Two transport implementations:
1. **LocalTransport** — crossbeam channels, in-process. Single-player mode.
2. **NetworkTransport** — quinn QUIC, over the wire. Multiplayer mode.

Server and client game systems only interact with `MessageInbox<M>` / `MessageOutbox<M>` resources. They never touch networking directly.

---

## Work Packages

### WP1: Workspace Scaffold + Empty Bevy Apps
**Goal:** `cargo run --bin client` opens a window. `cargo run --bin server` starts headless and logs. CI passes.

- Root `Cargo.toml` workspace with `crates/common`, `crates/server`, `crates/client`
- `common`: serde, bincode deps, placeholder lib.rs
- `server`: headless Bevy app (MinimalPlugins), logs startup
- `client`: Bevy app (DefaultPlugins), opens window with colored background
- `.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- `.gitignore`, `rustfmt.toml`
- `assets/` directory

**Test:** Both binaries run. CI green.

---

### WP2: Shared Types + Protocol
**Goal:** Core types and message protocol with serialization tests.

- `common/src/types.rs` — `TilePosition`, `EntityId`, `PlayerId`, `Direction` (8-dir), `Tick`
- `common/src/protocol.rs` — Message enums:
  - `ClientMessage`: `Login { username, password }`, `CreateAccount { username, password }`, `CreateCharacter { name }`, `MoveIntent { direction, tick }`, `Disconnect`
  - `ServerMessage`: `LoginSuccess { player_id }`, `LoginFailed { reason }`, `AccountCreated`, `CharacterCreated { character_id, spawn_position }`, `EnterWorld { tick, player_entity, position, map_id }`, `WorldSnapshot { tick, entities }`, `MoveConfirm { tick, position }`, `MoveReject { tick, position }`
  - `EntityState`: `entity_id, position, direction, move_state, sprite_id`
- `common/src/movement.rs` — Pure validation: `validate_move(from, direction, is_walkable) -> Option<TilePosition>`
- Unit tests: serde round-trips, movement validation

**Test:** `cargo test -p common` passes.

---

### WP3: Transport Abstraction + Local Transport
**Goal:** In-memory message passing between server and client plugins in one Bevy app.

- `common/src/transport.rs` — `MessageInbox<M>`, `MessageOutbox<M>` resources, `ConnectionId`
- `common/src/local_transport.rs` — `LocalTransportPlugin`: crossbeam channels, drains each frame
- `server/src/plugin.rs` — `ServerPlugin`: registers FixedUpdate at 20Hz, reads inbox, writes outbox. Handles Login/CreateAccount/CreateCharacter/MoveIntent.
- `client/src/plugin.rs` — `ClientPlugin`: sends messages, reads responses
- Client main.rs adds both ServerPlugin + ClientPlugin + LocalTransportPlugin for single-player

**Test:** Client sends Login, server responds. All in one process, no networking.

---

### WP4: Test Map + Tilemap Rendering
**Goal:** A Tiled map renders in the client window.

- Download LPC terrain tileset → `assets/sprites/terrain.png`
- Create test map in Tiled (30x20, 32x32 tiles): ground layer (grass/dirt/paths), collision layer (walls/trees), spawn point object
- Export as `assets/maps/test_map.tmj`
- `client/src/plugins/rendering.rs` — `RenderingPlugin`: loads tilemap via bevy_ecs_tiled, sets up layers
- `client/src/plugins/camera.rs` — `CameraPlugin`: 2D camera, appropriate zoom for 32px tiles
- `common/src/map.rs` — `CollisionMap` (HashSet<TilePosition> of blocked tiles), shared between client and server

**Test:** `cargo run --bin client` shows the tilemap. Collision data loaded.

---

### WP5: Account/Character Creation + Walking Around (Single-Player)
**Goal: THE MILESTONE.** Launch the game → create account → create character → spawn on map → walk around.

**Server-side (plugin.rs):**
- Simple account storage (in-memory HashMap for now, SQLite later)
  - `CreateAccount` → hash password (argon2), store, respond `AccountCreated`
  - `Login` → verify password, respond `LoginSuccess` or `LoginFailed`
  - `CreateCharacter` → create character record with name, assign spawn position from map data, respond `CharacterCreated`
  - After character selection → send `EnterWorld` with position and map
- Movement processing: validate MoveIntent against CollisionMap, send Confirm/Reject
- Entity collision: track occupied tiles, enforce blocking rules (players pass players, block monsters)
- 20Hz WorldSnapshot broadcast

**Client-side:**
- `client/src/plugins/input.rs` — WASD movement, sends MoveIntent + client-side prediction
- `client/src/plugins/rendering.rs` — Spawn player sprite (LPC 64x64), Y-sorting, smooth tile-to-tile interpolation, walk animation, camera follow
- `client/src/plugins/ui/` — Minimal UI flow:
  - **Login screen**: username + password fields, "Login" and "Create Account" buttons
  - **Character creation**: name field, "Create" button (minimal — no class selection yet, just getting in)
  - **In-game**: player sprite on map, basic position debug text
- Bevy states: `MainMenu` → `Login` → `CharacterSelect` → `InGame`
- Move queueing for fluid continuous movement
- Client prediction + reconciliation on MoveConfirm/MoveReject

**Assets:**
- LPC base character spritesheet → `assets/sprites/player.png`
- `assets/CREDITS.md` for LPC attribution

**Test:** Launch client. Create account "test"/"test". Create character "Hero". Spawn on grass map. Walk around with WASD, smooth movement, can't walk through walls/trees. Camera follows.

---

### WP6: Quinn Networking (Multiplayer)
**Goal:** Standalone server accepts network connections. Clients connect over QUIC.

- `server/src/plugins/network.rs` — `ServerNetworkPlugin`:
  - Tokio runtime in background thread
  - Quinn endpoint on `0.0.0.0:4433`
  - Self-signed TLS cert via rcgen
  - Bridge: tokio task ↔ crossbeam channels ↔ Bevy MessageInbox/Outbox
  - Reliable streams for auth/handshake, unreliable datagrams for position updates
- `client/src/plugins/network.rs` — `ClientNetworkPlugin`:
  - Mirror architecture, connects to server ip:port
  - Accept any cert in dev mode
- Client launch modes:
  - `--singleplayer` or default: LocalTransportPlugin (embed server)
  - `--connect <ip:port>`: ClientNetworkPlugin
- Message framing: 4-byte length prefix + bincode for reliable, raw bincode for datagrams

**Test:** Server in terminal A. Client with `--connect 127.0.0.1:4433` in terminal B. Login, create character, walk around — same experience as single-player but over the network.

---

### WP7: Multiple Players + Interpolation
**Goal:** Multiple connected players see each other with smooth interpolated movement.

- Server: WorldSnapshot includes all entity positions, sent to each client at 20Hz
- Client: for remote entities (not local player):
  - Buffer two most recent server positions
  - Interpolate between them each frame (render one tick behind)
  - Spawn/despawn remote sprites as they appear/disappear
- Y-sorting works correctly between multiple players and map objects

**Test:** Two clients connected to same server. Player A walks, player B sees smooth movement. Both Y-sorted correctly.

---

### WP8: Polish + Edge Cases
**Goal:** Harden everything. Handle edge cases. Document.

- Disconnect handling (server cleanup, client returns to login)
- Reject duplicate usernames, handle malformed messages
- Connection status text overlay (connecting/connected/disconnected)
- Debug overlay (F3): tile position, server tick, RTT, player count
- Move queue edge cases: rapid direction changes, diagonal movement
- Comprehensive unit tests for common crate
- Integration test: LocalTransport, send movement commands, assert state
- README.md: build instructions, how to run single-player, how to run multiplayer, asset setup

**Test:** All tests pass. Both modes work. Edge cases don't panic. README is complete.

---

## Dependency Graph

```
WP1 (scaffold)
 └─ WP2 (types/protocol)
     └─ WP3 (transport + local)
         ├─ WP4 (tilemap rendering)
         │   └─ WP5 (THE MILESTONE: account + character + walking)
         │       ├─ WP6 (quinn networking)
         │       │   └─ WP7 (multiplayer + interpolation)
         │       │       └─ WP8 (polish)
         │       └────────────┘
```

## Key Risks

| Risk | Mitigation |
|------|-----------|
| bevy_ecs_tiled incompatible with target Bevy version | Fall back to custom .tmj parser — Tiled JSON is straightforward |
| Quinn datagram API complexity | Start with reliable streams only, add datagrams as optimization |
| Y-sorting with tilemap layers | Test early in WP5, may need custom z-calculation to interleave entities with tile layers |
| Bevy FixedUpdate timing with interpolation | Use Bevy's `Time<Fixed>` for server, `Time<Virtual>` for client interpolation |

## Critical Files

- `crates/common/src/transport.rs` — architectural linchpin for single-player/multiplayer
- `crates/common/src/protocol.rs` — client/server message contract
- `crates/common/src/movement.rs` — shared movement validation (prediction + authority)
- `crates/server/src/plugin.rs` — server game loop (works embedded and standalone)
- `crates/client/src/plugins/rendering.rs` — tilemap, sprites, Y-sorting, interpolation

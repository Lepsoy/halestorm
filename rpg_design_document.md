# Halestorm — RPG Game Design Document

**Working Title:** Halestorm
**Version:** 0.3 — Design Complete
**Status:** Ready for Phase 1 Implementation
**Language:** Rust

---

## 1. Game Vision

A 2D tile-based online RPG inspired by Tibia's world and Guild Wars 1's build system, featuring a top-down perspective with LPC-style pixel art, modernized with lighting and particle effects, real-time combat with a limited skill bar, and an open hand-crafted world. Players explore, fight monsters, complete quests, collect loot, and craft builds from dual-class skill pools — all in a persistent world shared with a small group of friends.

### Core Pillars

- **Exploration-driven open world** — A hand-crafted world with distinct zones connected by ships, teleporters, and flight paths. Discovery is rewarding; the world is dense with secrets, lore, and hidden areas. Exploration is incentivized through stacking bonuses that are lost on death.
- **Build-crafting depth** — Inspired by Guild Wars 1: choose 8 skills from a large pool across two classes, distribute attribute points, and adapt your build to the challenge. Free respec in town encourages experimentation.
- **Meaningful real-time combat** — Energy-based skills with cooldowns and tactical positioning on a tile grid. Encounters should feel dangerous and require preparation.
- **Smooth modernized retro feel** — LPC-compatible top-down pixel art elevated with dynamic lighting, particle effects, and smooth interpolated movement. Classic soul, modern polish.
- **Small-server cooperative play** — Intimate online play for 2–20 players. No PvP. Cooperative dungeon crawling, trading, and shared exploration. Most content is solo-friendly; group content can be soloed when overleveled.

---

## 2. Technical Architecture

### 2.1 Engine & Framework

**Recommended: Bevy engine**

Bevy is a modern, open-source Rust game engine built on an ECS (Entity Component System) architecture. It provides 2D and 3D rendering, asset loading, audio, input handling, and a plugin ecosystem out of the box. Key reasons for choosing Bevy:

- **2D and 3D in one engine** — This project is 2D, but Bevy's renderer supports full 3D. Knowledge and patterns learned here carry directly into future 3D game projects without switching engines.
- **Rich ecosystem** — Active community with plugins for tilemaps, UI, and more.
- **Rust-native** — Idiomatic Rust, strong compile-time guarantees, good performance.
- **Fast iteration** — Built-in sprite rendering, asset hot-reloading, and ECS let you focus on game logic instead of engine plumbing.

**Bevy caveats to be aware of:**
- Pre-1.0 — API changes between versions (mitigated by pinning to a stable release).
- No built-in networking — multiplayer implemented via `quinn` with a custom protocol (see below).
- General-purpose renderer has overhead vs. a hand-tuned 2D pipeline, but this is negligible for a tile-based game.

**Complementary crates (used alongside Bevy):**

| Layer | Crate | Purpose |
|-------|-------|---------|
| Tilemap rendering | `bevy_ecs_tilemap` | Efficient tile map rendering and chunk management |
| Networking | `quinn` | QUIC-based networking with custom protocol. Provides reliable + unreliable channels over UDP with built-in encryption. Simple, stable, no framework coupling |
| Serialization | `serde` + `bincode` | Network messages and save data |
| Persistence | `rusqlite` | SQLite database for player and world state persistence |
| Tile map loading | `bevy_ecs_tiled` or custom parser | Load Tiled editor maps (.tmx/.tmj) into Bevy |
| UI (if Bevy's built-in UI is insufficient) | `bevy_egui` | Immediate-mode UI for debug/dev tools and game panels |

### 2.2 Platform Targets

**Primary target: Desktop (Windows, macOS, Linux)** — native binaries for both client and server via `cargo build`. This is the development and play platform for the foreseeable future.

**Future targets (separate effort, not blocking):**

| Platform | Approach | Notes |
|----------|----------|-------|
| Web (Browser) | Compile client to WASM via Bevy's WebGPU/WebGL2 support | Bevy has active WASM support; mainly a build/packaging task |
| Mobile (iOS, Android) | Native via Bevy's mobile support | Requires touch input layer and UI adaptation |

The architecture is designed so these ports are build-target changes, not rewrites. The server binary is always desktop/cloud only.

### 2.3 Client-Server Architecture

```
┌──────────────────┐         ┌──────────────────┐
│   Game Client     │ ◄─────► │   Game Server     │
│   (Bevy app)      │  UDP    │   (Headless Bevy) │
│                    │         │                    │
│  - Rendering       │         │  - Authoritative   │
│  - Input           │         │    game state      │
│  - Prediction      │         │  - Physics/collision│
│  - Interpolation   │         │  - Combat resolution│
│  - UI              │         │  - Loot/inventory   │
│  - Audio           │         │  - NPC/AI logic     │
│  - Local effects   │         │  - World persistence│
└──────────────────┘         └──────────────────┘
```

**Authoritative server model** — The server owns all game state. Clients send inputs (movement intent, attack commands, spell casts); the server validates and simulates. This prevents cheating and keeps all players in sync.

**Client-side prediction** — The client predicts the player's own movement locally for responsiveness, then reconciles with server corrections. Other players and monsters are interpolated between server snapshots.

**Network protocol (quinn / QUIC):**
- Custom message types defined in the `common` crate, serialized with `serde` + `bincode`.
- **Reliable streams** — Chat messages, inventory changes, quest updates, login/auth. QUIC provides ordered reliable delivery natively.
- **Unreliable datagrams** — Position updates, combat events, animation triggers. High frequency (~20 ticks/second from server). QUIC supports unreliable datagrams via the datagram extension.
- **Client-side prediction** for player movement is hand-rolled: predict tile transition locally, reconcile on server correction. Simple for tile-based movement.

**Four play modes**, all using the same server logic:
- **Single-player mode** — Server runs as an embedded Bevy plugin inside the client app. No separate process, no networking. Player clicks "Single Player" and plays immediately. Requires clean separation of client and server plugins (good architecture regardless).
- **LAN mode** — Standalone server binary bound to local network interface. Players connect via local IP.
- **Online mode (self-hosted)** — Standalone server on your machine with port forwarding or a tunnel service (e.g., ngrok, playit.gg). Free or very cheap.
- **Cloud mode** — Deploy the standalone server binary to a VPS (DigitalOcean, Hetzner, etc. ~$5–10/month). Always-on, public IP.

For multiplayer modes, players connect by entering `ip:port` in the client. No matchmaking infrastructure needed.

### 2.4 Tick Rate & Game Loop

- **Server tick rate:** 20 ticks/second (50ms per tick). Sufficient for tile-based real-time combat; keeps bandwidth low.
- **Client frame rate:** Unlocked (or vsync). Smooth interpolation between server ticks for visual fluidity.
- **Server loop per tick:** Process client inputs → Update game simulation (movement, combat, AI, cooldowns, spawns) → Build world snapshot delta → Send to clients.

### 2.5 Persistence

**SQLite** via the `rusqlite` crate. Single-file database, no separate database server required. Easy to back up (copy one file), easy to deploy.

**Stored data:**
- **Player data** — Account credentials, character stats, level, XP, attribute distribution, skill library, equipped skill bar, inventory, equipment, cosmetic unlocks, quest progress, home town binding.
- **World state** — Monster respawn timers, chest/container states, any world changes that need to persist across server restarts.

The database file lives alongside the server binary. Periodic auto-saves (e.g., every 5 minutes) plus save-on-player-disconnect.

### 2.6 Authentication

Simple system suitable for a small private server:

- Server has an optional **server password** (configured in server config file).
- Players connect with a **username** + server password.
- On first login with a new username, the server creates a player profile and prompts character creation.
- On subsequent logins, the server loads the existing player profile.
- Passwords are hashed (bcrypt or argon2) — never stored in plaintext.
- No email, no OAuth, no account recovery. Suitable for a small group of known players. Can be upgraded later if needed.

---

## 3. Rendering & Visual Style

### 3.1 Perspective

**Straight top-down perspective** (RPG Maker / classic Zelda style). The camera looks directly down. Depth is conveyed through Y-sorting, taller character sprites, and layered rendering — not through angled projection.

- Tiles are square (32×32 pixels for ground tiles).
- Characters use taller sprites (64×64) that overlap the tile behind them, creating a natural sense of depth via Y-sorting.
- Walls and elevation are conveyed through sprite layering and occlusion (e.g., walking behind a wall hides the character).
- No camera rotation — fixed angle.

### 3.2 Tile System

- **Tile size:** 32×32 pixels (ground). Matches LPC tileset standard.
- **Character sprite size:** 64×64 pixels (LPC standard). Characters are taller than one tile, giving the oblique perspective its sense of height.
- **Layers per tile:** Ground → Ground detail/decor → Objects/walls → Entities (players, monsters, NPCs) → Overhead (tree canopies, roofs that fade when player walks under).
- **Sprite sheets:** All tiles and sprites packed into texture atlases for efficient GPU rendering. Character sprite sheets generated via the LPC Spritesheet Generator.

### 3.3 Visual Enhancements (Beyond Classic Tibia)

- **Smooth sprite interpolation** — Characters glide between tiles rather than snapping.
- **Particle effects** — Spell impacts, torches, ambient dust, weather (rain, snow).
- **Light sources** — Torches, spells, and environmental lights cast colored light using a simple 2D lightmap overlay. Not a full dynamic shadow system — a per-tile light accumulation buffer blended over the scene.
- **Animated water and terrain** — Subtle tile animation for water, lava, swaying grass.
- **Screen shake and hit flash** — Juice for combat feedback.
- **Static global lighting** — No day/night cycle. Dungeons are dark; overworld is bright. Light sources in dungeons matter.

### 3.4 Render Pipeline (Bevy)

Bevy handles sprite batching and draw ordering natively. The rendering is organized as:

1. **Tile pass** — `bevy_ecs_tilemap` renders ground and detail layers from tilemap chunks efficiently.
2. **Entity pass** — Bevy sprites for characters, monsters, NPCs, objects. Y-sorted via Bevy's `Transform` z-ordering or a custom sorting system.
3. **Light pass** — Custom 2D lightmap: accumulate light sources into a render texture, multiply-blend over the scene. Implemented as a Bevy post-processing shader or overlay camera.
4. **Particle pass** — Bevy particle system (built-in or `bevy_hanabi` plugin) for spell effects, torches, ambient particles.
5. **UI pass** — Bevy UI or `bevy_egui` for HUD, panels, chat. Rendered last, unaffected by lighting.

---

## 4. World & Map Design

### 4.1 World Structure

The world is composed of **discrete zones** (maps), each a rectangular tilemap. Zones are connected by transition points:

- **Walking transitions** — Step on a tile at the edge of a zone to seamlessly load the adjacent zone.
- **Ships** — Travel between continents/islands. Triggered by NPC interaction at a dock.
- **Waypoints** — Discoverable fast-travel nodes placed throughout the world. Activated by visiting them. Players can teleport from any activated waypoint to any other activated waypoint. Every town has a waypoint. Placed at key locations in the field as well (dungeon entrances, crossroads, remote camps). Free to use.
- **Flight paths** — Unlockable fast-travel routes between discovered towns.

Each zone has a unique theme, tileset, and monster population. The world is fully hand-crafted using the **Tiled** map editor.

### 4.2 Multi-Floor Elevation (Tibia-style)

Each zone supports **multiple floors** (vertical layers). A zone is not just a flat tilemap — it is a stack of floors, each with its own tile layers, collision, entities, and lighting.

- **Floor numbering:** Ground level is floor 0. Floors above are +1, +2, etc. Floors below (basements, caves, sewers) are -1, -2, etc.
- **Stairs and ladders** — Transition tiles that move the player up or down one floor within the same zone. The transition is instant (no loading screen). This is a key tactical mechanic: **monsters do not follow players between floors** (with rare exceptions for special monsters). Stairs are an escape route — if overwhelmed, flee to the nearest staircase.
- **Per-floor visibility** — Only the current floor is rendered. Walking downstairs hides the surface; walking upstairs hides the basement. Floors are fully independent visual layers.
- **Per-floor collision** — Each floor has its own collision map. A wall on floor 0 does not affect floor 1.
- **Per-floor spawns** — Monsters spawn on specific floors. Deeper dungeon floors have harder monsters, creating vertical difficulty progression within a single zone.
- **Buildings and interiors** — A building's exterior is on floor 0. Its interior can be on floor 0 (entered via door transition) or on a separate floor (e.g., floor +1 for upper stories, floor -1 for cellars). This adds density to the world without expanding horizontally.
- **Tiled implementation** — Each floor is a separate Tiled layer group (or a separate .tmj file linked to the zone). Floor metadata (stair connections, floor number) is encoded as custom properties.

### 4.3 Narrative Approach

**No central main quest or endgame goal.** Like Tibia, each zone and region has its own self-contained questlines and stories. Players discover lore and narrative through exploration, NPC dialogue, and zone-specific quest chains. The world grows organically as new zones are added, each bringing their own story. This keeps the game open-ended and avoids a "finished the story, now what?" problem.

### 4.4 Map Editor Pipeline

```
Tiled Editor (.tmx/.tmj)
    ↓
Export JSON/XML
    ↓
Rust map loader (tiled crate or custom parser)
    ↓
In-game zone data: tile layers, collision layer, spawn points,
NPC positions, transition points, light sources, object metadata
```

Tiled supports custom properties on tiles and objects, which can encode game-specific data: spawn types, NPC dialogue keys, quest triggers, door locks, etc.

### 4.5 Zone Expansion

New zones are added by creating new maps in Tiled and connecting them to existing zones via transition tiles or travel NPCs. The server loads all zone definitions at startup. This makes the game easy to expand — add a map file and a connection, and the new content is live.

### 4.6 Collision & Pathfinding

- **Terrain collision:** Tile-based. Each tile is either walkable or blocked (with per-layer collision flags). Simple and fast.
- **Entity collision rules:**
  - **Players pass through players** — no blocking between players. Prevents griefing and pathing frustration.
  - **Players and monsters block each other** — a player cannot walk through a monster, and a monster cannot walk through a player. This makes positioning tactical: a Champion can tank a doorway, but getting surrounded is genuinely dangerous. Movement skills (Charge, Evasive Roll) serve as escape tools.
  - **Monsters block monsters** — monsters cannot stack on the same tile. Creates natural-looking movement and tactical bottlenecks.
- **Simultaneous move conflicts:** When two blocking entities try to move to the same tile on the same tick, the server resolves by priority: the entity whose move was received first wins; the other's move is rejected and they remain on their original tile.
- **Pathfinding (server-side):** A* on the tile grid for monster AI, accounting for both terrain and entity blocking. Players don't need server-side pathfinding since they send direct movement inputs.

---

## 5. Movement System

### 5.1 Tile-Based with Smooth Interpolation

The game world is logically a tile grid. A character's **authoritative position** is always a tile coordinate (or transitioning between two tile coordinates). However, visually, the sprite smoothly interpolates between tiles.

**How it works:**
- Player presses WASD (8-directional movement).
- Client sends movement intent to server.
- Server validates: is the target tile walkable? If yes, the character begins transitioning.
- The transition takes a fixed duration (e.g., 150ms per tile, tunable by movement speed stat).
- Client interpolates the sprite position between the source and target tile over that duration.
- **Consistent movement speed** — diagonal movement takes √2 times longer than cardinal movement (Tibia-style), so the character moves at the same speed in all directions. No shortcuts by zigzagging diagonally.
- Player can queue the next move before the current transition completes, enabling fluid continuous movement.

### 5.2 Movement Speed

Movement speed is a character stat (base + equipment bonuses). It controls the tile transition duration. Possible speed buffs from spells, skills, and equipment, or debuffs from terrain (e.g., swamp tiles slow movement).

---

## 6. Combat System

### 6.1 Overview

Real-time, skill-based combat on the tile grid. Players equip a bar of 7 regular skills + 1 ultimate chosen from their two class pools and use them in combat fueled by a fast-recharging energy system. Combat is PvE only — no player-vs-player combat.

### 6.2 Skill Bar

Inspired by Guild Wars 1's build system:

- Players have access to a large pool of skills across their primary and secondary class.
- Before leaving town, players select **7 skills** for their regular slots plus **1 ultimate skill** for the dedicated ultimate slot. This is their loadout for the field.
- **Skills cannot be changed outside of town.** This makes build preparation a strategic decision — you need to anticipate what you'll face.
- Skills are bound to hotkeys (1–8).
- Each skill has: energy cost, cooldown (recharge time), cast time (some instant), range, and effect.

### 6.3 Energy System

- **Energy** is the universal resource for all skills (replaces mana).
- Energy has a maximum pool (e.g., 40–80 depending on class/attributes) and **regenerates quickly** — roughly 3–4 energy per second at base rate. Champions have lower base energy regeneration than other classes, compensated by the Momentum system.
- This means players are always casting, always active. Energy management is about pacing and prioritization, not conservation.
- Some skills or effects can drain, steal, or boost energy regeneration — creating counterplay in PvE encounters.

**Momentum (Champion only):**
- Secondary resource unique to the Champion class. A player-level resource (not per-target).
- **Max 10 stacks.** Built by landing melee hits — auto-attacks and melee skills each grant 1 stack.
- **Decays after ~10 seconds out of combat** — momentum resets if the Champion stops fighting.
- **Finisher skills** consume all current momentum stacks, with damage/effect scaling based on stacks consumed. More momentum = bigger payoff.
- Creates a distinct combat rhythm: sustain with attacks to build momentum, then unleash powerful finishers. Compensates for lower energy regen by rewarding sustained aggression.

### 6.4 Attack Types

- **Melee skills** — Hit targets on adjacent tiles (8 directions). Some melee skills may hit multiple adjacent tiles (cleave).
- **Ranged skills** — Projectiles travel across tiles to hit a target. Line-of-sight check required.
- **AoE skills** — Affect a pattern of tiles (circle, cone, line). May require clicking a tile to place the effect.
- **Support skills** — Healing, buffs, condition removal. Can target self or allies.
- **Auto-attack** — A basic attack that fires automatically at the selected target between skill uses. Damage is low; skills are the primary source of damage and utility.

### 6.5 Targeting

- **Click to target** — Click a monster to select it as your target.
- **Auto-attack** — Fires on cooldown at current target when no skill is being used.
- **Skill targeting** — Single-target skills fire at current target. AoE skills may require clicking a tile to place the effect. Self/ally skills target appropriately.

### 6.6 Cast Bars

- **Player cast bar** — Prominent bar displayed near the character or HUD when casting any skill with a cast time. Shows skill name, cast progress, and can be interrupted by movement or stun effects.
- **Monster cast bars** — Visible above monster sprites when they are casting abilities. Allows observant players to react — interrupt, dodge out of AoE, or brace for impact. This is a core skill expression mechanic: reading enemy cast bars and responding is what separates good play from great play.
- **Instant skills** — No cast bar shown. Fire immediately on keypress.
- **Channeled skills** — Show a cast bar that drains rather than fills. Interrupted if the player moves or is stunned.

### 6.7 Healing Model

Multiple healing sources, all viable:

- **Slow natural regeneration** — HP regenerates slowly out of combat. Very slow or paused during combat.
- **Health potions** — Consumable items with a shared cooldown (e.g., one potion every 15 seconds). Provides a moderate heal. Good as an emergency supplement.
- **Healing skills** — Class skills (especially from healer-oriented classes) are the primary in-combat healing. Can be self-targeted or ally-targeted.

### 6.8 Combat Stats

| Stat | Effect | Source |
|------|--------|--------|
| Health (HP) | Damage taken before death | Leveling (primary), gear |
| Energy | Resource pool for skills | Class/attributes (mostly fixed) |
| Energy Regen | Energy recovered per second | Class/attributes, gear |
| Attack Power | Base damage for physical skills and auto-attacks | Leveling, gear |
| Magic Power | Base damage/effectiveness for magical skills | Leveling, gear |
| Defense | Reduces incoming physical damage | **Gear only** |
| Magic Resist | Reduces incoming spell damage | **Gear only** |
| Movement Speed | Tile transition speed | Base + gear + buffs |

Defense and magic resist come **exclusively from gear**. This means armor choice is a core part of character identity: a tank in plate has high defense, a mage in robes is squishy but compensates with skills (healing, kiting, conditions). High-level characters do not passively become tanky — they must gear for it.

Critical chance, armor penetration, condition duration, and other secondary stats may be added as attribute lines and gear provide them, but the core stats above drive the system.

### 6.9 Conditions & Boons

Skills can apply **conditions** (debuffs) and **boons** (buffs):

- **Conditions** (examples): Bleeding (damage over time), Weakness (reduced attack), Cripple (slowed movement), Blind (attacks miss), Burning (magic damage over time).
- **Boons** (examples): Regeneration (heal over time), Might (increased damage), Swiftness (faster movement), Protection (reduced damage taken).
- Some skills specialize in applying, removing, or converting conditions and boons — creating build synergies.

### 6.10 Death & Consequences

Death is **consequential but not punishing**:

- On death, the player loses all **blessings** and **exploration bonuses** (see Section 7.5).
- The player is teleported back to their **home town** (bindpoint).
- **No XP loss. No item loss. No equipment degradation.**
- The sting of death comes from losing accumulated field bonuses — the deeper you were into a dangerous area with stacked blessings, the more it hurts to die. This creates natural tension without frustrating players.

### 6.11 Difficulty & Group Scaling

- **Most content is solo-friendly** at the intended level range.
- **Group content** (hard dungeons, bosses) is designed for parties at-level but can be **soloed when overleveled** — like Tibia, raw stats can overcome mechanics designed for coordination.
- Monster difficulty is tuned per zone. No dynamic scaling — the world has a fixed difficulty gradient that players progress through.

---

## 7. Character & Progression

### 7.1 Leveling & XP

- Characters gain experience (XP) by killing monsters and completing quests.
- **Monsters always give their fixed XP value regardless of player level.** A goblin worth 50 XP is always worth 50 XP whether you're level 5 or level 500. Content is never "devalued" — it just becomes inefficient as XP requirements grow.
- Each level requires progressively more XP (exponential curve). This is the only form of devaluation.
- **No level cap.** Players can level indefinitely, always making progress.
- Leveling grants:
  - **Increased base HP** — The primary survival scaling. More levels = bigger health pool.
  - **Increased base attack power and magic power** — More levels = more damage.
  - **Attribute points** — Up to a fixed cap (see 7.3). Once the cap is reached, further levels still grant HP and offensive stats but no more attribute points.
- Leveling does **NOT** grant:
  - Defense or magic resist (gear only).
  - Significant energy pool increases (energy stays roughly fixed by class/attributes).

### 7.2 Classes & Dual-Classing

**6 base classes:**

**Champion** — Melee frontline fighter.
- Three weapon-driven playstyles: **sword & shield** (tank — mitigation, taunts, blocking), **two-handed** weapons (big deliberate hits, wide sweeping AoE, slower and harder-hitting), **dual wield** (fast aggressive attacks, frequent low-damage AoE, high haste, button-mashy).
- Movement abilities — charges, leaps, gap-closers. Gets into the fight fast.
- Shouts — warcry buffs for allies, intimidation debuffs on enemies.
- Berserker skills — risk/reward abilities trading defense for offense, possibly stronger at low HP.
- Purely physical — no magic whatsoever. Lower base energy regen than other classes, compensated by the **Momentum** system (build stacks through melee hits, spend on finishers).
- Weapons: 1H swords/axes/maces + shield, 2H greatswords/hammers/greataxes, dual wield 1H weapons.
- Armor: heavy plate.
- Attribute lines:
  - **Power** (primary exclusive) — Generic damage/stat amplifier boosting all Champion skills. Ultimate: **Earthshatter** — Punch the ground, shockwave in 3-tile radius. Enemies within 1 tile stunned 4s, 2 tiles stunned 3s, 3 tiles stunned 2s. 3 min CD.
  - **Toughness** (shield) — Shield and defensive skills. Ultimate: **Shield Wall** — reduce damage taken by 50% for 10 seconds. 3 min CD.
  - **Berserking** (2H) — Two-handed power skills. Ultimate: **Massive Sweep** — sequential cone attacks: forward, backward, right, left. 3 min CD.
  - **Slaying** (dual wield) — Fast dual wield skills. Ultimate: **Frenzy** — increases melee haste by 50% for 10 seconds. 3 min CD.

**Ranger** — Ranged physical damage / survivalist.
- Thrown weapons, traps, explosives, and incendiaries. No pets.
- **1H thrown weapon + shield** (axes, knives, javelins, stars — tankier, defensive) or **2H bows/crossbows** (higher ranged damage, no shield).
- Traps — snares, spike traps, placed on tiles for area denial.
- Explosives/incendiaries — fire bombs, AoE grenades, burning ground. Some of the best physical AoE in the game.
- Very limited magic — minor nature-based regeneration and self-healing. Self-sufficient in the field but not a healer for others.
- Armor: medium (leather/mail).
- Attribute lines:
  - **Windrunning** (primary exclusive) — Increases dodge chance and movement speed. Ultimate: **Zephyr** — +50% dodge chance for 20 seconds. Each successful dodge stacks +10% movement speed, up to +100%. 3 min CD.
  - **Sniper** — Single target damage, increased range, crit chance. Ultimate: **Ballista** — 2 second cast, fires a massive arrow piercing through all monsters in a line to the target. 3 min CD.
  - **Survival** — Nature regen, cleansing, self-buffs. Ultimate: **Athelas Weed** — powerful regeneration effect. 3 min CD.
  - **Poacher** — Traps and explosives. Ultimate: **Explosive Shot** — charges arrows with explosives, hitting in a + pattern around the target (5 tiles). 3 min CD.

**Monk** — Holy warrior / healer.
- Two distinct playstyles: **healer build** (protection prayers, condition removal, healing allies, boons — the group support backbone) or **melee damage build** (holy smiting fists, sacred strikes, divine punishment — glass cannon in close range).
- Healing is divine light; damage is smiting energy channeled through fists.
- Melee damage monks survive through self-healing and smart play despite low armor.
- Weapons: fist weapons, knuckles, brass knuckles, cesti only. No shields.
- Armor: light (robes/cloth).
- Attribute lines:
  - **Divinity** (primary exclusive) — Every cast triggers bonus healing. Offensive skills give self-heal; support/healing skills get extra heal on target. Ultimate: **Ascendance** — Massively amplifies Divinity's bonus healing (3-5x) on all casts for 15 seconds. 3 min CD.
  - **Martial Arts** — Melee haste and damage. Ultimate: **Furious Fists** — 5 rapid hits, each targeting a new enemy in melee range (or all on same target if solo). Hits scale 1x, 2x, 3x, 4x, 5x damage. 3 min CD.
  - **Grace** — Healing skills. Ultimate: **Angels' Descent** — large heal on all party members. 3 min CD.
  - **Faith** — Support and buffs. Ultimate: **Divine Aegis** — absorption shield on a single target scaling with spell power and attributes. 3 min CD.

**Elementalist** — Elemental burst caster.
- Three elements, each with a distinct tactical identity:
  - **Fire** — Raw damage. Fireballs, meteors, fire walls, burning DoTs. AoE heavy. The iconic "blow things up" element.
  - **Lightning** — Fast, sharp damage. Chain lightning, thunderbolts, stuns/interrupts. More single-target and reactive.
  - **Earth** — Defensive/control. Create blocking terrain (rock walls, pillars on tiles), tremors, knockdowns. Reshape the battlefield.
- Elements combo naturally: block enemies in with earth, then rain fire. Use earth defensively while zapping with lightning.
- Weapons: staves, or wand + off-hand focus/orb.
- Armor: light (robes/cloth).
- Attribute lines:
  - **Radiance** (primary exclusive) — Bigger energy pool, energy return from auto-attacks (wanding). Ultimate: **Overflow** — All skills cost 0 energy for 15 seconds. 3 min CD.
  - **Terrestrial** (earth) — Terrain control, blocking, knockdowns. Ultimate: **Fissure** — directed line forward, heavy damage, stuns for 5 seconds, leaves impassable terrain for 15 seconds. 3 min CD.
  - **Inferno** (fire) — Raw AoE damage, burning. Ultimate: **Cataclysm** — massive AoE with high damage. 3 min CD.
  - **Thunder** (lightning) — Fast damage, stuns, interrupts. Ultimate: **Tesla Strike** — huge single target nuke that AoE stuns around the target. 3 min CD.

**Illusionist** — Control / disruption specialist. Inspired by the GW1 Mesmer.
- Interrupts — punish enemies for using skills, shut down dangerous abilities.
- Energy denial — drain enemy resources, slow their output.
- Domination/punishment — hex-like effects that trigger when enemies act (attack, cast, move). The more they do, the more they suffer.
- Illusions/confusion — misdirection, enemies waste attacks or hit wrong targets.
- Very cerebral, high skill ceiling. Doesn't deal huge direct damage but makes everything easier for the group.
- Weapons: staves, or wand + off-hand focus/orb.
- Armor: light (robes/cloth).
- Attribute lines:
  - **Spellslinger** (primary exclusive) — Reduced cast times. Ultimate: **Quicksilver** — All skills become instant cast for 10-15 seconds. 3 min CD.
  - **Saccading** — Interrupts, micro-stuns on actions, stutter effects. Ultimate: **Silence** — AoE silence preventing all enemies in area from using skills. 3 min CD.
  - **Intrusion** — DoTs, hexes, debilitating effects. Ultimate: **Nightmare** — powerful single target DoT hex. 3 min CD.
  - **Illusions** — Misdirection, making mobs attack each other, decoys that draw aggro. Ultimate: **Mind Control** — take control of a single enemy, turning it against its allies. 3 min CD.

**Cultist** — DoT / condition / ritual specialist.
- Diseases & afflictions — spreading DoTs that rot enemies. Poison, plague, decay. Can potentially spread between enemies.
- Rituals — powerful effects with long cast times. Cast in combat (risky, need protection) or as preparation before engaging. Some rituals buff the party at a cost (sacrifice HP, reduce energy regen, etc.).
- Corpse skills — explode enemy corpses for AoE damage or effects. The battlefield becomes a resource.
- Channeled auras — toggle/channel effects radiating around the Cultist. Forces them into close/mid range despite being squishy. Risk/reward positioning.
- Life drain / self-sustain — stays alive by leeching health from afflicted enemies. The more things are rotting, the healthier the Cultist.
- Dark, unholy, sacrificial magic — the opposite of the Monk's divine light.
- Weapons: staves, scythes, or wand + off-hand focus/orb.
- Armor: light (robes/cloth).
- Attribute lines:
  - **Occultism** (primary exclusive) — Dark arts mastery. Active DoTs increase spell power, empowered corpse skills (bonus damage, larger radius), faster ritual cast times. Ultimate: **Exorcism** — For 15-20 seconds, every skill cast forces a worm out of the target which explodes for AoE damage. 3 min CD.
  - **Vampyrism** — Life drain abilities, self-sustain. Ultimate: **Nosferatu** — doubles healing from life drain abilities for 20 seconds. 3 min CD.
  - **Fanatism** — Auras and rituals. Ultimate: **Reaper's Rite** — out-of-combat ritual that empowers the scythe to deal conical splash damage on hit. 3 min CD.
  - **Decay** — DoTs, diseases, weakening. Ultimate: **Black Death** — strong damage that also weakens target's melee damage by 30%. 3 min CD.

---

**Dual-Classing:**

**Primary class** is chosen at character creation and is **permanent**. It grants:
- Access to all class skills, including **ultimate skills** (one ultimate equipped at a time in a dedicated ultimate slot).
- A **primary attribute line** — a unique attribute only available to characters with this as their primary class. See each class definition above for details.
- Core class identity.

**Skill bar: 7 regular skills + 1 ultimate slot = 8 total.** The ultimate slot is locked to ultimate skills only. One ultimate at a time. All ultimates have a 3 minute cooldown.

**Secondary class** is unlocked through gameplay (quest chain or milestone). It grants:
- Access to the secondary class's regular skills (but NOT its ultimate skills).
- Access to the secondary class's regular attribute lines (but NOT its primary attribute line).
- The secondary class can be **changed** by visiting a class trainer in town.

**Dual-class restrictions:** Each class has **3 available secondary classes** (not all 5). This keeps combinations designable and ensures each pairing is viable.

| Primary | Available Secondaries |
|---------|----------------------|
| Champion | Monk, Cultist, Illusionist |
| Ranger | Illusionist, Cultist, Elementalist |
| Monk | Illusionist, Champion, Ranger |
| Elementalist | Illusionist, Cultist, Ranger |
| Illusionist | Cultist, Elementalist, Monk |
| Cultist | Champion, Monk, Elementalist |

This yields **18 unique primary/secondary combinations**. The identity of each combo emerges naturally from which skills the player puts on their 8-slot bar. A Champion/Monk plays very differently from a Monk/Champion because of different primary attributes and ultimate skills.

### 7.3 Attribute System

**Hybrid model:** a few universal attributes plus class-specific attribute lines. Attributes act as **multipliers** on skill effectiveness — they shape your build, not your raw power (that comes from levels).

**Attribute point cap:** Players earn attribute points from leveling up to a fixed cap (e.g., attribute points stop at level 50 or 80, even though leveling continues forever). This means a level 200 player and a level 80 player may have the same attribute distribution, but the level 200 has vastly higher base stats from levels. Attributes define *how* you're powerful; levels define *how much*.

**Universal attributes** (all classes have these):
- Energy Pool — increases max energy (the primary way to get more energy, since leveling doesn't grant it).
- Energy Recovery — increases energy regeneration rate.

**Class attribute lines** (4 per class: 1 primary exclusive + 3 regular):

| Class | Primary Exclusive | Line 2 | Line 3 | Line 4 |
|-------|------------------|--------|--------|--------|
| Champion | Power | Toughness (shield) | Berserking (2H) | Slaying (dual wield) |
| Ranger | Windrunning | Sniper | Survival | Poacher |
| Monk | Divinity | Martial Arts | Grace | Faith |
| Elementalist | Radiance | Terrestrial (earth) | Inferno (fire) | Thunder (lightning) |
| Illusionist | Spellslinger | Saccading | Intrusion | Illusions |
| Cultist | Occultism | Vampyrism | Fanatism | Decay |

- Attribute points are distributed freely across available lines (primary class lines + secondary class lines + universal).
- Each skill is linked to an attribute line. Investing points in that line increases the skill's effectiveness (damage, duration, healing amount, etc.).
- Primary exclusive lines are only available to characters with that class as primary — secondary class users cannot access them.
- **Free respec in town** — attribute points can be redistributed at any time while in a town. Encourages experimentation and adapting builds to content.

### 7.4 Skills & Skill Acquisition

Players have access to a large pool of skills per class. Skills are acquired through multiple channels:

- **Skill trainers** — NPCs in towns teach core skills for gold. New skills become available as the player levels up.
- **Quest rewards** — Some skills are rewards for completing specific quests, encouraging exploration.
- **Exploration / discovery** — Rare or elite skills found in hidden locations, dropped by bosses, or unlocked by completing challenges. These are the prestige skills that reward thorough players.

All acquired skills go into the player's **skill library** — a permanent collection. From this library, the player selects 7 for their regular skill slots + 1 ultimate for the ultimate slot before leaving town.

### 7.5 Starter Skills

Each attribute line has a set of core skills. Below are 3 starter skills per line — enough to build functional early-game loadouts. All skills scale with their linked attribute line. Skills marked with a cast time show a cast bar; instant skills fire immediately.

**CHAMPION**

| Skill | Line | Type | Cast | CD | Energy | Effect |
|-------|------|------|------|----|--------|--------|
| Shield Bash | Toughness | Melee, single target | Instant | 8s | 5 | Hits adjacent target, stuns for 2s |
| Fortify | Toughness | Self buff | Instant | 20s | 5 | Reduces damage taken by 20% for 8s |
| Shield Charge | Toughness | Movement, melee | Instant | 15s | 10 | Rush forward up to 4 tiles, knocking back the first enemy hit |
| Cleave | Berserking | Melee, cone | Instant | 6s | 5 | Heavy swing hitting all enemies in a 3-tile frontal cone |
| Overhead Slam | Berserking | Melee, single target | 0.5s | 10s | 8 | Massive single hit with knockdown |
| Reckless Fury | Berserking | Self buff | Instant | 25s | 5 | +30% damage, -15% defense for 10s |
| Twin Slash | Slaying | Melee, AoE | Instant | 4s | 4 | Quick double hit on all adjacent enemies |
| Flurry | Slaying | Melee, single target | Instant | 8s | 6 | 4 rapid strikes on target |
| Blade Storm | Slaying | Melee, AoE | Instant | 12s | 8 | Spin attack hitting all 8 adjacent tiles |
| War Cry | Power | Shout, party buff | Instant | 30s | 10 | +15% attack power to self and nearby allies for 10s |
| Charge | Power | Movement | Instant | 12s | 5 | Rush to target up to 6 tiles away, closing the gap instantly |
| Battle Roar | Power | Shout, debuff | Instant | 20s | 8 | Intimidate: enemies within 3 tiles deal 10% less damage for 8s |

**RANGER**

| Skill | Line | Type | Cast | CD | Energy | Effect |
|-------|------|------|------|----|--------|--------|
| Aimed Shot | Sniper | Ranged, single target | 1.0s | 6s | 5 | High damage shot with bonus crit chance |
| Piercing Arrow | Sniper | Ranged, line | 0.5s | 10s | 8 | Arrow pierces through enemies in a line |
| Mark Prey | Sniper | Ranged, debuff | Instant | 20s | 5 | Mark a target: all attacks against it deal +15% damage for 10s |
| Herbal Remedy | Survival | Self heal | 1.0s | 15s | 8 | Moderate self heal |
| Nature's Vigor | Survival | Self buff | Instant | 25s | 5 | Regeneration: heal over time for 12s |
| Cleanse | Survival | Self buff | Instant | 12s | 5 | Remove 2 conditions from self |
| Bear Trap | Poacher | Trap, single tile | Instant | 10s | 5 | Place on tile. First enemy to step on it takes damage and is immobilized for 3s |
| Fire Bomb | Poacher | Ranged, AoE | Instant | 12s | 8 | Throw explosive dealing AoE damage in a + pattern and leaving burning ground for 4s |
| Scatter Mines | Poacher | Trap, AoE | 1.0s | 20s | 10 | Place 3 mines in a cluster. Each detonates on contact for physical AoE damage |
| Evasive Roll | Windrunning | Movement | Instant | 8s | 5 | Dodge-roll 2 tiles in facing direction, brief invulnerability during roll |
| Tailwind | Windrunning | Self buff | Instant | 25s | 5 | +30% movement speed for 8s |
| Wind Shot | Windrunning | Ranged, single target | Instant | 10s | 6 | Quick shot that knocks the target back 2 tiles |

**MONK**

| Skill | Line | Type | Cast | CD | Energy | Effect |
|-------|------|------|------|----|--------|--------|
| Palm Strike | Martial Arts | Melee, single target | Instant | 4s | 4 | Fast melee hit |
| Roundhouse Kick | Martial Arts | Melee, cone | Instant | 8s | 6 | Kick hitting all enemies in frontal cone |
| Inner Fire | Martial Arts | Self buff | Instant | 20s | 5 | +25% melee haste for 8s |
| Healing Touch | Grace | Ranged, single target heal | 1.0s | 6s | 8 | Moderate heal on self or ally |
| Mending Wave | Grace | Ranged, AoE heal | 1.5s | 15s | 12 | Heal self and all nearby allies |
| Purify | Grace | Ranged, single target | Instant | 10s | 5 | Remove 2 conditions from target ally |
| Blessing of Light | Faith | Ranged, single target buff | Instant | 20s | 8 | Target ally gains +20% damage for 10s |
| Protective Ward | Faith | Ranged, AoE buff | 1.0s | 25s | 10 | All nearby allies gain +15% defense for 8s |
| Holy Ground | Faith | AoE, placed | 1.0s | 20s | 10 | Consecrate a 3x3 area: allies inside regenerate HP for 10s |
| Smite | Divinity | Melee, single target | Instant | 6s | 5 | Holy damage strike with bonus self-heal |
| Divine Radiance | Divinity | AoE, around self | 0.5s | 15s | 8 | Burst of holy light damaging enemies and healing allies nearby |
| Absolution | Divinity | Ranged, single target | Instant | 20s | 10 | Heal an ally and deal damage to their attacker |

**ELEMENTALIST**

| Skill | Line | Type | Cast | CD | Energy | Effect |
|-------|------|------|------|----|--------|--------|
| Fireball | Inferno | Ranged, single target | 0.75s | 5s | 6 | Classic fireball, moderate damage |
| Flame Burst | Inferno | Ranged, AoE | 1.0s | 10s | 10 | Explosion at target tile, hitting 3x3 area |
| Fire Wall | Inferno | Placed, line | 1.0s | 15s | 8 | Create a line of fire across 5 tiles, burning enemies that cross for 6s |
| Lightning Bolt | Thunder | Ranged, single target | Instant | 5s | 6 | Fast bolt dealing high single target damage |
| Chain Lightning | Thunder | Ranged, chain | 0.5s | 10s | 10 | Hits target then chains to 2 nearby enemies for reduced damage |
| Thunderclap | Thunder | AoE, around target | 0.75s | 12s | 8 | Shock at target location, interrupting all enemies in 3x3 area |
| Stone Wall | Terrestrial | Placed, blocking | 1.0s | 18s | 8 | Create a line of 3 impassable rock tiles lasting 10s |
| Tremor | Terrestrial | AoE, around self | 0.75s | 12s | 8 | Earthquake in 5x5 area around caster, damage + knockdown |
| Boulder Throw | Terrestrial | Ranged, single target | 1.0s | 8s | 6 | Hurl a boulder at target, dealing damage and stunning for 1.5s |
| Arcane Bolt | Radiance | Ranged, single target | Instant | 4s | 3 | Low cost, low cooldown energy bolt. Returns 2 energy on hit |
| Elemental Surge | Radiance | Self buff | Instant | 25s | 5 | +20% spell power for 8s |
| Mana Siphon | Radiance | Ranged, single target | Instant | 15s | 0 | Drain energy from target, recovering 15 energy |

**ILLUSIONIST**

| Skill | Line | Type | Cast | CD | Energy | Effect |
|-------|------|------|------|----|--------|--------|
| Power Spike | Saccading | Ranged, single target | Instant | 6s | 5 | Interrupt target's current cast, dealing bonus damage if a cast was interrupted |
| Daze | Saccading | Ranged, single target | Instant | 10s | 6 | Stun target for 2s |
| Cry of Frustration | Saccading | AoE, around self | Instant | 15s | 10 | Interrupt all enemies within 3 tiles |
| Empathy | Intrusion | Ranged, hex | 1.0s | 10s | 8 | Hex: target takes damage each time it attacks. Lasts 10s |
| Backfire | Intrusion | Ranged, hex | 1.0s | 10s | 8 | Hex: target takes damage each time it casts a skill. Lasts 10s |
| Migraine | Intrusion | Ranged, hex/DoT | 1.0s | 15s | 10 | Hex: damage over time + reduces target's energy regeneration by 50%. Lasts 10s |
| Phantom | Illusions | Placed, decoy | 1.0s | 20s | 10 | Create an illusion at target tile that draws enemy aggro for 8s |
| Confusion | Illusions | Ranged, single target | 0.75s | 15s | 8 | Target attacks its nearest ally for 5s |
| Mirror Image | Illusions | Self buff | Instant | 25s | 8 | Create 2 copies of self that absorb one hit each before vanishing |
| Quick Cast | Spellslinger | Self buff | Instant | 20s | 5 | Next 3 skills cast 50% faster |
| Feedback | Spellslinger | Ranged, single target | Instant | 12s | 6 | Hit target and drain 10 energy from it |
| Shatter | Spellslinger | Ranged, AoE | Instant | 15s | 8 | Destroy your active illusions/phantoms, each exploding for AoE damage |

**CULTIST**

| Skill | Line | Type | Cast | CD | Energy | Effect |
|-------|------|------|------|----|--------|--------|
| Life Siphon | Vampyrism | Ranged, single target | Instant | 5s | 5 | Deal damage and heal self for 50% of damage dealt |
| Blood Pact | Vampyrism | Self buff | Instant | 20s | 5 | Sacrifice 15% HP: next 3 attacks drain life for full damage dealt |
| Vampiric Aura | Vampyrism | Aura, channel | Instant | 25s | 3/s | Channel: nearby allies heal for a portion of damage they deal. Drains energy per second |
| Dark Ritual | Fanatism | Ritual, party buff | 3.0s | 30s | 15 | Long cast. Party gains +20% spell power for 30s. Caster loses 10% max HP while active |
| Aura of Decay | Fanatism | Aura, channel | Instant | 20s | 3/s | Channel: enemies within 3 tiles take damage over time |
| Unholy Fervor | Fanatism | Self buff, ritual | 2.0s | 25s | 10 | Ritual: +25% cast speed for 15s |
| Plague Touch | Decay | Melee, DoT | Instant | 6s | 5 | Infect adjacent target with disease dealing damage over 10s. Can spread to nearby enemies on kill |
| Wither | Decay | Ranged, single target DoT | 0.75s | 8s | 6 | Curse that deals damage over 8s and slows movement by 20% |
| Pestilence | Decay | Ranged, AoE DoT | 1.5s | 15s | 12 | Spread disease in a 3x3 area, all targets take damage over 8s |
| Corpse Blast | Occultism | AoE, corpse | Instant | 8s | 5 | Explode a nearby corpse dealing AoE damage in a 3x3 area |
| Soul Harvest | Occultism | Self buff | Instant | 20s | 0 | Consume up to 3 nearby corpses, restoring 8 energy each |
| Dark Pact | Occultism | Self buff | Instant | 25s | 0 | Sacrifice 20% HP to gain +30% spell power for 10s |

*These are starter/core skills. Additional skills will be acquired through trainers, quests, and exploration. Each attribute line will eventually have 8–12+ skills to choose from.*

### 7.5 Blessings & Exploration Bonuses

This is the system that gives death its weight without permanent penalties:

**Blessings:**
- Obtained from shrines, temples, and blessing NPCs scattered across the world.
- Each blessing provides a meaningful buff (e.g., +10% damage, +15% XP gain, +20% gold find, reduced energy costs).
- Multiple blessings can be stacked — collecting all available blessings in a region makes you significantly more powerful.
- **All blessings are lost on death.**

**Exploration bonus:**
- A stacking bonus that builds as you venture deeper into dangerous territory without dying.
- The further from town and the more dangerous the zone, the faster the bonus accumulates.
- The bonus could increase XP gain, loot quality, or provide flat stat increases.
- **Entirely lost on death.** This creates a risk/reward loop: push deeper for bigger bonuses, but death erases your streak.

**Temporary consumable buffs** (food, potions, scrolls):
- Provide short-duration buffs (e.g., 30 minutes of increased stats).
- **Also lost on death** — the buff timer is wiped, not just paused.
- Players can re-apply them after respawning, but the cost adds up.

### 7.6 Equipment

Equipment is the **primary source of defense** and a secondary source of offensive power. Build and skill choices matter more than gear for damage output, but gear is critical for survivability.

- **Equipment slots:** Head, chest, legs, feet, main hand, off hand, ring, amulet.
- **Defense and magic resist come exclusively from gear.** Armor type defines your survivability profile:
  - Heavy armor (plate) — high defense, low magic resist, minimal offensive bonuses. Tank/melee oriented.
  - Medium armor (leather/mail) — balanced defense and magic resist. Hybrid/ranged oriented.
  - Light armor (robes/cloth) — low defense, moderate magic resist, offensive/utility bonuses. Caster oriented.
- Weapons provide attack power or magic power bonuses, and may have special effects.
- Accessories (rings, amulets) provide attribute bonuses, energy bonuses, or special effects.
- **Level requirements** and possible class restrictions on some items.
- **Rarity tiers:** Common, uncommon, rare, epic, legendary (or a custom naming scheme).
- Equipment is **not lost on death**.
- **Gear is NOT visible on the character sprite** — equipment is stat-only, visible in the equipment panel. Character appearance is controlled by the cosmetic system (see 7.7).
- A well-built character with average gear outperforms a poorly-built character with great gear — but a well-built character with great gear is the goal.

### 7.7 Character Customization & Cosmetics

Inspired by Tibia's outfit system. The character's visual appearance is entirely separate from their equipped gear.

**Character creation:**
- Choose a base body type, skin tone, hair style, hair color.
- Choose starting outfit (limited selection).

**Cosmetic unlocks (earned through gameplay):**
- **Outfits** — Complete visual looks unlocked by completing quests, achievements, or milestones. Each outfit is a curated sprite set (not mix-and-match armor pieces). Examples: complete a dungeon to earn a dark knight outfit, reach a hidden island to unlock a tribal outfit, defeat a dragon boss for a flame-themed outfit.
- **Accessories** — Capes, hats, auras, wings, shoulder pieces layered on top of the current outfit. Earned through specific accomplishments.
- **Body types / variants** — Additional character models or variations unlocked through gameplay.
- **Color customization** — Outfits may support recoloring (primary/secondary color channels) so players can personalize unlocked outfits.

**Design benefits:**
- Seeing a rare outfit on another player immediately signals their accomplishments — cosmetics are a status symbol.
- Art pipeline is simpler: outfits are complete sprites, not modular armor pieces that need to layer on every body type.
- No pressure to make gear visually impressive — gear is purely a stats game.
- Encourages quest completion and exploration for cosmetic rewards.

---

## 8. Town vs. Field

A core design distinction borrowed from Guild Wars 1: **towns are safe hubs; the field is where the game happens.**

### 8.1 In Town

- **Safe zone** — No monsters, no danger, no death.
- **Build management** — Change your 8 equipped skills, redistribute attribute points, switch secondary class. All free, all instant.
- **Services** — NPC shops, skill trainers, quest givers, blessing NPCs, travel NPCs, storage/bank.
- **Social hub** — Chat, trade with other players, form parties.
- **Home town** — Each player has a bound home town (changeable). This is where you respawn on death.

### 8.2 In the Field

- **Skill bar is locked** — The 8 skills you left town with are what you have. No swapping.
- **Attributes are locked** — No respec outside town.
- **Blessings and exploration bonuses accumulate** — Rewarding continued play without dying.
- **Death is consequential** — Lose blessings, lose exploration bonus, teleport back to town.
- **Monsters, quests, loot, bosses** — All content is in the field.

This creates a satisfying loop: prepare in town → venture into the field → push as far as you can → return (or die) → adjust build → go again.

### 8.3 Housing

Each character has a **preset personal house** in their home town. No buying, selling, or competing for plots. No dynamic decoration system. The house serves as a personal space and potential storage point. May be expanded in the future.

### 8.4 Mounts

**No mounts.** Movement speed is handled through buffs, skills (e.g., Ranger's Windrunning), and equipment bonuses.

---

## 9. NPCs, Quests & Lore

### 9.1 NPCs

- NPCs are placed on the map via the Tiled editor with custom properties defining their type and data references.
- **Dialogue trees** — NPC interaction uses a click-through dialogue tree system. Players are presented with dialogue and choose from response options that branch the conversation.
- NPC types: quest givers, shopkeepers (buy/sell items), skill trainers (teach skills for gold), blessing NPCs, lore providers, travel NPCs (ships, teleporters), and flavor/ambient characters.

### 9.2 Quests

- Quests are defined in data files (RON or JSON) — not hardcoded.
- Quest types: kill X monsters, deliver item, explore area, defeat boss, solve puzzle, escort NPC.
- Quests can gate access to new zones, unlock the secondary class, reward rare skills, or provide unique equipment.
- Quest log tracks active and completed quests.

### 9.3 Monsters

- Monsters spawn in defined zones at defined spawn points (configured in Tiled).
- Each monster type has: stats, loot table, AI behavior pattern, respawn timer, skill set.
- AI behaviors: patrol, chase when player enters aggro range, flee at low HP, call for help, use skills/spells on cooldown.
- Boss monsters with unique mechanics and guaranteed/rare loot.
- Monster difficulty is fixed per zone — no scaling. The world has a clear difficulty gradient.

---

## 10. Inventory & Economy

### 10.1 Inventory

- Grid-based or slot-based inventory (to be decided).
- Weight or slot limit.
- Equipment slots: head, chest, legs, feet, main hand, off hand, ring, amulet.

### 10.2 Economy

- Monsters drop gold and items.
- NPC shops for buying/selling basics.
- Player-to-player trading (trade window).
- Possibly a player marketplace or auction board in towns.

---

## 11. UI & Controls

### 11.1 HUD Layout

```
┌─────────────────────────────────────────────────────────┐
│                                          ┌──────────┐   │
│                                          │ Minimap  │   │
│                                          └──────────┘   │
│                                          ┌──────────┐   │
│           GAME VIEWPORT                  │ Side     │   │
│                                          │ Panel    │   │
│     (characters have HP bars             │ (Bags,   │   │
│      and condition icons                 │  Char,   │   │
│      above their sprites)                │  Attrs,  │   │
│                                          │  Quests) │   │
│                                          │          │   │
│                                          └──────────┘   │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  [Player nameplate]    [Target nameplate]       │    │
│  │  HP ████████░░  Energy ██████░░░░               │    │
│  │  [Player cast bar ████████░░░░░░░░░░░░░░░░░░░]  │    │
│  │  [ 1 ][ 2 ][ 3 ][ 4 ][ 5 ][ 6 ][ 7 ][ ULT ]  │    │
│  │  [              Chat window                   ] │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

**Game viewport** — Center of screen, shows the world.

**In-world elements (above sprites):**
- **HP bars above all characters and enemies** — Small bars floating above every entity. Visible at all times.
- **Condition/buff icons** — Small icons displayed in a row below or beside the HP bar:
  - **Green** — Boons/buffs (regeneration, might, protection, etc.)
  - **Red** — Damage conditions (bleeding, burning)
  - **Yellow** — Control conditions (weakness, blind)
  - **Brown** — Movement conditions (cripple, slow)
  - **Purple** — Hex/curse conditions (energy drain, punishment hexes)
- **Enemy cast bars** — Visible above enemy sprites when they are casting. Players can react to these (interrupt, dodge, brace).
- **Player cast bars are NOT visible to other players** — Only you see your own cast bar, displayed in the HUD (not above your sprite).
- **Nameplates** — Character names above sprites for players and named NPCs.

**Bottom HUD panel:**
- **Player nameplate** — Left side, shows name, level, HP bar, energy bar, and active conditions/boons.
- **Target nameplate** — Right side (or next to player), shows selected target's name, level, HP bar, and active conditions/boons. Only appears when a target is selected.
- **Player cast bar** — Prominent bar between nameplates and skill bar. Shows skill name, cast progress. Only visible during casting.
- **Skill bar** — 7 equipped skills (1–7 keys) + 1 ultimate slot. Each slot shows the skill icon, cooldown overlay, and energy cost. Greyed out when on cooldown or insufficient energy.
- **Chat window** — Below the skill bar, text input for communication.

**Right side panel (Tibia-style):**
- Always-visible side panel on the right edge of the screen.
- Contains toggleable sub-panels: **Inventory/Bags**, **Character sheet** (stats, equipment), **Attribute distribution**, **Skill library**, **Quest log**.
- Stays open while playing — no need to pause or overlay the game viewport.
- **Minimap** — Top of the side panel or top-right corner of the viewport.

### 11.2 Controls

| Input | Action |
|-------|--------|
| WASD | Move (8-directional) |
| Left click | Target entity / interact with NPC / pick up item |
| Right click | Context menu (inspect, attack, trade) |
| 1–7 | Skill bar (7 equipped skills) |
| 8 or dedicated key | Ultimate skill |
| Enter | Open chat input |
| Tab | Cycle nearest targets |
| Escape | Close panels / open menu |

Side panel tabs are clickable or bound to hotkeys. The side panel is always accessible without closing the game viewport.

Mobile: virtual joystick for movement, tap to target, skill bar buttons for skills.

---

## 12. Data-Driven Design

The game should be heavily data-driven to allow content expansion without recompiling:

| Data | Format | Purpose |
|------|--------|---------|
| Maps | Tiled .tmj (JSON) | Zone layouts, collision, spawn points, transitions |
| Monsters | RON or JSON | Stats, loot tables, AI type, skill set, sprite reference |
| Items | RON or JSON | Stats, rarity, equip slot, description, sprite |
| Skills | RON or JSON | Damage, cooldown, energy cost, effect type, linked attribute, animation |
| Quests | RON or JSON | Objectives, rewards, prerequisites, dialogue references |
| NPC Dialogue | RON or JSON | Dialogue trees (nodes, options, branches, conditions) |
| Blessings | RON or JSON | Buff effects, shrine locations, stacking rules |
| Classes | RON or JSON | Attribute lines, primary attribute, available skills per level |
| Config | TOML | Server tick rate, movement speed, XP curve, energy regen, network settings |

**RON (Rusty Object Notation)** is a Rust-friendly data format that supports enums and structs natively. Good for game data definitions. JSON is the alternative if tooling compatibility matters more.

---

## 13. Engineering Principles

**Code quality and maintainability are the top priority.** Features should be adjusted or simplified if doing so results in cleaner, more maintainable code. A well-structured codebase that's easy to refactor is worth more than a feature-rich mess.

### 13.1 Core Principles

- **Composition over inheritance** — Use Bevy's ECS to compose behavior from small, focused components. Avoid deep hierarchies or god-objects.
- **Separation of concerns** — Each system does one thing. Combat logic doesn't touch rendering. Networking doesn't know about UI. Systems communicate through components and events.
- **Easy to refactor** — Code should be structured so that moving, renaming, or restructuring modules is straightforward. Loose coupling between systems. Minimal cross-module dependencies.
- **Data-driven over hardcoded** — Game content (skills, monsters, items, quests) lives in data files, not in Rust code. Adding content should never require recompiling game logic.
- **Shared logic in common crate** — Any logic needed by both client and server (combat formulas, movement validation, type definitions, protocol) lives in the `common` crate. Never duplicate logic between client and server.
- **Small, focused modules** — Prefer many small files over few large ones. A 500-line file should be split. Each file should have a clear, single responsibility.
- **Explicit over clever** — Readable, obvious code is preferred over elegant abstractions. Comments explain *why*, not *what*.
- **Pin Bevy, upgrade deliberately** — Bevy is pre-1.0 and breaking changes between versions are expected. Pin to a specific release and only upgrade when needed (bug fixes, crate compatibility). Budget time for upgrade work when it happens.

### 13.2 Testing Strategy

- **Unit tests** — `common` crate is the priority: damage calculations, stat scaling, skill effects, condition/boon logic, movement validation. These are pure functions with no engine dependencies — fast and easy to test.
- **Integration tests** — Client-server interaction: connection handshake, movement synchronization, combat resolution, inventory changes. Run a headless server and simulated client in-process.
- **Data validation tests** — All data files (skills, monsters, items, quests, dialogues) must parse correctly, reference valid attribute lines, and contain no broken references. Run on every test pass.
- **CI merge pipeline** — All tests run on every merge request via CI (GitHub Actions). Merge is blocked if any test fails. Pipeline also runs `cargo clippy` and `cargo fmt --check`.

### 13.3 Bevy ECS Patterns

- **Components are data-only structs** — No logic in components. They hold state.
- **Systems are functions** — Each system queries for components it needs and operates on them. Keep systems small and focused.
- **Events for cross-system communication** — When one system needs to trigger behavior in another (e.g., combat system sends a DamageEvent, UI system listens for it), use Bevy events.
- **Plugins for feature modules** — Each major feature (combat, movement, inventory, UI, networking) is a Bevy plugin that registers its own systems, components, and events. Plugins can be enabled/disabled cleanly.
- **States for game flow** — Use Bevy states for game flow control (Loading, MainMenu, InGame, Paused). Systems run only in their relevant states.
- **Resources for global singletons** — Game config, asset handles, network connection state — these are Bevy resources, not components.

---

## 14. Project Structure

```
halestorm/
├── crates/
│   ├── common/              # Shared types, protocol, game logic
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── protocol.rs      # Network message types
│   │   │   ├── types.rs         # Shared game types (Position, EntityId, etc.)
│   │   │   ├── combat.rs        # Combat formulas, damage calculation
│   │   │   ├── skills.rs        # Skill definitions and effects
│   │   │   ├── stats.rs         # Stat calculation, attribute scaling
│   │   │   ├── conditions.rs    # Condition/boon definitions and logic
│   │   │   └── map.rs           # Map data structures and tile definitions
│   │
│   ├── server/              # Headless game server (Bevy app)
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── plugins/
│   │   │   │   ├── world.rs         # Zone management, transitions
│   │   │   │   ├── network.rs       # Client connection handling
│   │   │   │   ├── ai.rs            # Monster AI systems
│   │   │   │   ├── combat.rs        # Server-side combat resolution
│   │   │   │   ├── spawning.rs      # Monster/NPC spawning
│   │   │   │   ├── player.rs        # Player session state
│   │   │   │   ├── quest.rs         # Quest state tracking
│   │   │   │   ├── persistence.rs   # SQLite save/load
│   │   │   │   └── auth.rs          # Authentication
│   │
│   ├── client/              # Game client (Bevy app)
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── plugins/
│   │   │   │   ├── rendering.rs     # Sprite rendering, Y-sorting, tilemap
│   │   │   │   ├── input.rs         # Input handling, keybinds
│   │   │   │   ├── network.rs       # Server connection, prediction, interpolation
│   │   │   │   ├── camera.rs        # Viewport and camera logic
│   │   │   │   ├── audio.rs         # Sound/music
│   │   │   │   └── ui/
│   │   │   │       ├── mod.rs       # UI plugin root
│   │   │   │       ├── hud.rs       # HP/energy bars, skill bar, cast bar
│   │   │   │       ├── nameplates.rs # In-world nameplates, HP bars, conditions
│   │   │   │       ├── side_panel.rs # Tibia-style side panel
│   │   │   │       ├── dialogue.rs  # NPC dialogue tree UI
│   │   │   │       ├── chat.rs      # Chat window
│   │   │   │       └── menus.rs     # Main menu, settings, character creation
│
├── assets/
│   ├── sprites/             # Sprite sheets and texture atlases (LPC)
│   ├── maps/                # Tiled map files (.tmj)
│   ├── audio/               # Sound effects and music
│   └── data/                # Game data files (RON/JSON)
│       ├── skills/          # Skill definitions per class
│       ├── monsters/        # Monster definitions
│       ├── items/           # Item/equipment definitions
│       ├── quests/          # Quest definitions
│       ├── dialogues/       # NPC dialogue trees
│       ├── blessings.ron    # Blessing definitions
│       ├── classes.ron      # Class and attribute line definitions
│       └── config.toml      # Server/client configuration
│
├── tools/                   # Map converter, asset packer, admin tools
├── Cargo.toml               # Workspace manifest
└── README.md
```

---

## 15. Development Phases

### Phase 1 — Core Engine & Networking
- Bevy app setup, window creation, basic 2D rendering.
- Load and display a Tiled map with `bevy_ecs_tilemap`.
- Client-server connection via `quinn` with custom message protocol.
- Player movement with server authority and client-side prediction.
- Basic sprite rendering and Y-sorting.

### Phase 2 — World & Entities
- Multi-zone world with walking transitions.
- Monster spawning, basic AI (chase, patrol).
- Collision detection on tile grid.
- NPC placement and basic dialogue tree interaction.

### Phase 3 — Combat & Skills
- Energy system with regeneration.
- Skill bar (8 slots) with cooldowns and energy costs.
- Melee, ranged, and AoE skill execution.
- Auto-attack system.
- Damage calculation, health bars, conditions/boons.
- Death → lose blessings → teleport to town.
- Combat particle effects and screen feedback.

### Phase 4 — Character Progression
- Class selection at creation.
- Attribute system (universal + class lines) with free respec in town.
- Skill acquisition from trainers, quests, and exploration.
- Secondary class unlock quest.
- Leveling and XP curve.
- Inventory and equipment system.
- Loot drops and item definitions.

### Phase 5 — World Systems
- Blessing shrines and exploration bonus system.
- NPC shops, skill trainers, travel NPCs.
- Quest system with quest log.
- Zone transitions (ships, teleporters, flight paths).

### Phase 6 — Polish
- Lighting system (lightmap).
- Audio (ambient, SFX, music).
- Chat system.
- UI polish and menus.

### Phase 7 — Content Expansion
- Design and build world zones.
- Populate with monsters, NPCs, quests, blessings.
- Balance combat, skills, and economy.
- Boss encounters with unique mechanics.

### Phase 8 — Platform Ports (Future)
- WASM build and browser testing.
- Mobile input layer and UI adaptation.

---

## 16. Art Pipeline

### 16.1 Asset Source: Liberated Pixel Cup (LPC)

All game art is sourced from the **Liberated Pixel Cup (LPC)** project and compatible assets on OpenGameArt.org. LPC is a large body of compatible, free pixel art created by a community of artists. License: **CC-BY-SA 3.0** (free to use, must credit artists).

**Why LPC:**
- Massive library of compatible assets designed to work together in a consistent style.
- Character sprites include hundreds of clothing, armor, weapon, and accessory options.
- Matching tilesets for environments (towns, dungeons, forests, interiors, etc.).
- Monster and creature sprites available in the same style.
- Completely free — no cost, no royalties. Only requirement is attribution.

### 16.2 Character Sprites

Characters are generated using the **Universal LPC Spritesheet Generator** (https://liberatedpixelcup.github.io/Universal-LPC-Spritesheet-Character-Generator/).

- Character sprites are **64×64 pixels** on a 32×32 tile grid (characters are taller than one tile, creating depth through Y-sorted overlap).
- The generator produces sprite sheets with animations: walk, run, idle, spellcast, slash, thrust, shoot, hurt, and more.
- Body types, heads, hair, clothing, armor, and weapons can be mixed and matched.
- For the cosmetic outfit system: each unlockable outfit is a pre-generated sprite sheet combining specific clothing/armor/accessory layers. Players don't mix armor visually in-game — outfits are curated complete looks.
- A **Rust crate (`lpcg`)** exists for programmatic sprite sheet generation, which could be used to batch-generate outfit variants.

### 16.3 Tilesets & Environment Art

- LPC-compatible tilesets from OpenGameArt.org for terrain, buildings, dungeons, interiors, and props.
- 32×32 ground tiles matching the LPC character style.
- All tilesets loaded via the Tiled editor pipeline (see Section 4.4).

### 16.4 Monsters & NPCs

- LPC-compatible monster sprites from OpenGameArt.org.
- NPCs use the same LPC character generator with different outfit combinations.
- Boss monsters may need larger sprites (128×128 or custom) — source from OpenGameArt or create custom.

### 16.5 Spell Effects & Particles

- Simple particle effects can be created programmatically (Bevy particle system).
- Additional effect sprites (explosions, magic impacts, projectiles) sourced from OpenGameArt.
- These don't need to match LPC style precisely since they're transient visual effects.

### 16.6 UI Art

- UI elements (buttons, panels, frames, icons) sourced from OpenGameArt or created as simple vector/pixel art.
- Skill icons: sourced from free icon packs on OpenGameArt or itch.io. Many fantasy RPG icon sets are available.

### 16.7 Attribution

LPC requires crediting all contributing artists. The spritesheet generator can auto-generate a credits file listing all artists whose assets were used. This credits file should be included in the game (accessible from the main menu) and in any distributed builds.

---

## 17. Open Design Decisions

The following need further discussion and will be expanded in future revisions:

1. **Specific skills** — Design the regular skill list per class. Skill archetypes, synergies, signature skills.
2. **Monster design** — Specific monster types, behaviors, and how they challenge different builds.
3. **Boss mechanics** — What makes boss fights interesting beyond stat checks.
4. **Guilds** — Guild system, shared guild hall, cooperative bonuses.
5. **Economy balance** — Gold sinks, item value curves, preventing inflation on small servers.
6. **Exploration bonus formula** — How fast does it stack, what does it affect, how does zone danger factor in.

---

*This is a living document. Each section will be expanded as design decisions are made.*

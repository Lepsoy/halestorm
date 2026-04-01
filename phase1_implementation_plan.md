# Halestorm — Implementation Plan (Updated)

## Current State

Phase 1 milestone achieved and significantly exceeded. The game has:
- Persistent accounts (SQLite + argon2), multiple characters per account
- Class selection (6 classes, per-class LPC sprites)
- Login → Character Select → Character Create → In Game UI flow
- 100x100 map with town, forest, ruins, lake, dungeon zones
- WASD + Q/E/Z/C movement with server-authoritative collision
- Diagonal movement with Tibia-style sqrt(2) timing
- LPC walk animation system (4-direction, 9 frames)
- Monsters: goblin spawning, AI (idle/chase/return), entity collision
- Smooth interpolation for all remote entities
- Transport abstraction ready for multiplayer (LocalTransport implemented)
- 29 tests, clippy clean

## What's Next: Combat & Skills Phase

### WP-C1: A* Pathfinding for Monsters
**Goal:** Monsters navigate around obstacles to reach the player instead of walking straight and getting stuck.

- Implement A* on the tile grid in `common/src/pathfinding.rs`
- Account for terrain collision + entity blocking (other monsters)
- Monsters find paths around walls, through doorways, around each other
- Monsters surround the player from multiple sides instead of queuing behind each other
- Path recalculation when target moves or path is blocked
- Limit path length to leash range

**Test:** Lure a monster around a wall. Multiple monsters should approach from different sides.

---

### WP-C2: Health System + HP Bars
**Goal:** Players and monsters have visible health. Foundation for damage.

- Player HP: base from level (start at 100), displayed in HUD
- Monster HP: already in protocol (EntityState has hp/max_hp)
- HP bars rendered above all entities (green bar, red background)
- Player HP/energy bars in bottom HUD panel
- Server tracks player HP in CharacterData, persists to DB

**Test:** HP bars visible above player and monsters.

---

### WP-C3: Targeting + Auto-Attack
**Goal:** Click a monster to target it. Auto-attack fires at the target.

- Click-to-target: left click on a monster selects it as target
- Target nameplate: show target's name, HP bar, level in HUD
- Target highlight: visual indicator on selected entity
- Auto-attack: if target is in melee range (adjacent tile), deal damage automatically on a timer (~1.5s)
- Server-side damage calculation: attack power vs defense
- Monster death: remove entity, drop to 0 HP, respawn after timer
- Player death: lose blessings (future), teleport to spawn

**Protocol additions:**
- `ClientMessage::SetTarget { entity_id }`
- `ClientMessage::AutoAttack`
- `ServerMessage::DamageDealt { source, target, amount }`
- `ServerMessage::EntityDied { entity_id }`
- `ServerMessage::PlayerDied`

**Test:** Click goblin, auto-attack kills it. Goblin respawns after a few seconds.

---

### WP-C4: Monster Combat AI
**Goal:** Monsters fight back. Getting surrounded is dangerous.

- Monsters deal damage to adjacent players on a timer
- Aggro table: monsters prioritize the player who hit them
- Multiple monsters can attack the same player simultaneously
- Monsters that can't reach melee range try to reposition (uses A* from WP-C1)
- Combat logging: damage numbers shown briefly above entities

**Test:** Walk into a group of goblins. They surround you and deal damage. You can die.

---

### WP-C5: Energy System + Skill Bar
**Goal:** Energy resource, skill bar UI, first skills.

- Energy: pool (40-80 by class), fast regen (~3-4/sec), displayed in HUD
- Skill bar: 7 slots + 1 ultimate slot, bound to 1-8 keys
- Skill data: loaded from RON/JSON data files
- Skill execution: client sends SkillUse, server validates (energy, cooldown, range), applies effect
- Cast bars: visible during cast time, interruptible by movement
- Cooldown overlays on skill bar icons

**First skills to implement (2 per class starter):**
- Champion: Cleave (melee AoE), War Cry (party buff)
- Monk: Palm Strike (melee), Healing Touch (heal)
- Elementalist: Fireball (ranged), Lightning Bolt (ranged)

**Test:** Press skill key, energy drains, cooldown starts, damage/healing applied.

---

### WP-C6: Conditions & Death Consequences
**Goal:** Buffs/debuffs, death penalty system.

- Conditions: bleeding, burning, regeneration, might, etc.
- Condition icons displayed below HP bars
- Death: teleport to town spawn, lose exploration bonuses (future)
- Monster respawning with configurable timer per spawn point
- Out-of-combat HP regeneration

**Test:** Get hit by burning attack, see DoT ticking. Die, respawn in town.

---

### WP-C7: Experience & Leveling
**Goal:** Monsters give XP on kill. Characters level up with visible feedback.

- XP per monster: fixed value per MonsterKind (goblins = 50 XP). No scaling by player level — per design doc.
- Exponential XP curve: each level requires more XP than the last. No level cap.
- Level up grants: increased base HP, increased attack power and magic power, attribute points (up to cap)
- Level stored in DB, displayed in HUD and character select
- Visual/audio feedback on level up (flash, text popup)
- Monster XP values defined in data (monster.rs or data files)

**Test:** Kill goblins, see XP bar fill, level up, HP increases.

---

## Future Work (After Combat Phase)

### Multiplayer (WP6-WP8 from original plan)
- Quinn QUIC networking
- Multiple players seeing each other
- Standalone server binary

### World Expansion
- Multi-floor elevation (Tibia-style stairs)
- More zones, NPCs, quests
- Secondary class system
- More monster types and boss encounters

### Polish
- Audio (ambient, SFX, music)
- Lighting system
- Chat system
- Debug overlay
- README and documentation

## Critical Files

- `crates/common/src/protocol.rs` — client/server message contract
- `crates/common/src/movement.rs` — shared movement validation
- `crates/common/src/monster.rs` — monster type definitions and stats
- `crates/server/src/plugins/game.rs` — server game loop, combat resolution
- `crates/server/src/plugins/monsters.rs` — monster AI and spawning
- `crates/server/src/plugins/persistence.rs` — SQLite database
- `crates/client/src/plugins/entities.rs` — remote entity rendering + interpolation
- `crates/client/src/plugins/animation.rs` — LPC sprite animation
- `crates/client/src/plugins/player.rs` — local player movement + rendering
- `crates/client/src/plugins/input.rs` — WASD + skill input

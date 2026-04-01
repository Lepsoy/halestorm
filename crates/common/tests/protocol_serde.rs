use halestorm_common::protocol::*;
use halestorm_common::types::*;
#[allow(unused_imports)]
use halestorm_common::types::PrimaryClass;

fn roundtrip_client(msg: &ClientMessage) -> ClientMessage {
    let bytes = bincode::serialize(msg).expect("serialize");
    bincode::deserialize(&bytes).expect("deserialize")
}

fn roundtrip_server(msg: &ServerMessage) -> ServerMessage {
    let bytes = bincode::serialize(msg).expect("serialize");
    bincode::deserialize(&bytes).expect("deserialize")
}

#[test]
fn client_login_roundtrip() {
    let msg = ClientMessage::Login {
        username: "player1".into(),
        password: "secret".into(),
    };
    let decoded = roundtrip_client(&msg);
    match decoded {
        ClientMessage::Login { username, password } => {
            assert_eq!(username, "player1");
            assert_eq!(password, "secret");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn client_create_account_roundtrip() {
    let msg = ClientMessage::CreateAccount {
        username: "newuser".into(),
        password: "pass123".into(),
    };
    let decoded = roundtrip_client(&msg);
    match decoded {
        ClientMessage::CreateAccount { username, password } => {
            assert_eq!(username, "newuser");
            assert_eq!(password, "pass123");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn client_move_intent_roundtrip() {
    let msg = ClientMessage::MoveIntent {
        direction: Direction::NorthEast,
        tick: Tick(42),
    };
    let decoded = roundtrip_client(&msg);
    match decoded {
        ClientMessage::MoveIntent { direction, tick } => {
            assert_eq!(direction, Direction::NorthEast);
            assert_eq!(tick, Tick(42));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn server_enter_world_roundtrip() {
    let msg = ServerMessage::EnterWorld {
        tick: Tick(100),
        entity_id: EntityId(1),
        position: TilePosition::new(10, 20),
        map_id: "test_map".into(),
        class: PrimaryClass::Monk,
    };
    let decoded = roundtrip_server(&msg);
    match decoded {
        ServerMessage::EnterWorld {
            tick,
            entity_id,
            position,
            map_id,
            class,
        } => {
            assert_eq!(tick, Tick(100));
            assert_eq!(entity_id, EntityId(1));
            assert_eq!(position, TilePosition::new(10, 20));
            assert_eq!(map_id, "test_map");
            assert_eq!(class, PrimaryClass::Monk);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn server_world_snapshot_roundtrip() {
    let msg = ServerMessage::WorldSnapshot {
        tick: Tick(200),
        entities: vec![
            EntityState {
                entity_id: EntityId(1),
                position: TilePosition::new(5, 5),
                direction: Direction::South,
                moving: false,
                class: PrimaryClass::Champion,
            },
            EntityState {
                entity_id: EntityId(2),
                position: TilePosition::new(8, 3),
                direction: Direction::West,
                moving: true,
                class: PrimaryClass::Elementalist,
            },
        ],
    };
    let bytes = bincode::serialize(&msg).expect("serialize");
    let decoded: ServerMessage = bincode::deserialize(&bytes).expect("deserialize");
    match decoded {
        ServerMessage::WorldSnapshot { tick, entities } => {
            assert_eq!(tick, Tick(200));
            assert_eq!(entities.len(), 2);
            assert_eq!(entities[0].entity_id, EntityId(1));
            assert_eq!(entities[1].moving, true);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn server_login_failed_roundtrip() {
    let msg = ServerMessage::LoginFailed {
        reason: "invalid password".into(),
    };
    let decoded = roundtrip_server(&msg);
    match decoded {
        ServerMessage::LoginFailed { reason } => {
            assert_eq!(reason, "invalid password");
        }
        _ => panic!("wrong variant"),
    }
}

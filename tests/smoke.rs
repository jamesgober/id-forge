use id_forge::nanoid;
use id_forge::snowflake::Snowflake;
use id_forge::ulid::Ulid;
use id_forge::uuid::Uuid;

#[test]
fn smoke_uuid_v4() {
    let id = Uuid::v4();
    assert_eq!(id.to_string().len(), 36);
}

#[test]
fn smoke_uuid_v7() {
    let id = Uuid::v7();
    assert_eq!(id.to_string().len(), 36);
}

#[test]
fn smoke_uuid_unique() {
    assert_ne!(Uuid::v4(), Uuid::v4());
}

#[test]
fn smoke_ulid_length() {
    assert_eq!(Ulid::new().to_string().len(), 26);
}

#[test]
fn smoke_ulid_unique() {
    assert_ne!(Ulid::new(), Ulid::new());
}

#[test]
fn smoke_ulid_monotonic() {
    let a = Ulid::new();
    let b = Ulid::new();
    assert!(b > a);
}

#[test]
fn smoke_ulid_roundtrip() {
    let id = Ulid::new();
    assert_eq!(Ulid::parse_str(&id.to_string()).unwrap(), id);
}

#[test]
fn smoke_snowflake_unique() {
    let gen = Snowflake::new(1);
    assert_ne!(gen.next_id(), gen.next_id());
}

#[test]
fn smoke_snowflake_worker_extracts() {
    let gen = Snowflake::new(42);
    let id = gen.next_id();
    let (_, worker, _) = Snowflake::parts(id);
    assert_eq!(worker, 42);
}

#[test]
fn smoke_snowflake_try_next_id_ok() {
    let gen = Snowflake::new(1);
    let id = gen.try_next_id().expect("clock should advance");
    assert!(id > 0);
}

#[test]
fn smoke_snowflake_monotonic_burst() {
    let gen = Snowflake::new(1);
    let mut prev = gen.next_id();
    for _ in 0..5_000 {
        let cur = gen.next_id();
        assert!(cur > prev);
        prev = cur;
    }
}

#[test]
fn smoke_nanoid_default() {
    let id = nanoid::generate();
    assert_eq!(id.len(), 21);
}

#[test]
fn smoke_nanoid_custom_length() {
    let id = nanoid::with_length(8);
    assert_eq!(id.len(), 8);
}

#[test]
fn smoke_nanoid_custom_alphabet() {
    let id = nanoid::custom(16, b"01");
    assert!(id.chars().all(|c| c == '0' || c == '1'));
}

#[test]
fn smoke_nanoid_try_custom_ok() {
    let id = nanoid::try_custom(12, b"0123456789").unwrap();
    assert_eq!(id.len(), 12);
    assert!(id.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn smoke_nanoid_try_custom_rejects_duplicate() {
    assert!(nanoid::try_custom(8, b"aabb").is_err());
}

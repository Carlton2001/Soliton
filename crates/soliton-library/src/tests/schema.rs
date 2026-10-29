use super::*;

#[test]
fn a_migration_leaving_inconsistent_references_keeps_the_previous_version() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys=OFF;")
        .unwrap();
    connection
        .execute_batch(include_str!("../schema_v1.sql"))
        .unwrap();
    let error = step(&mut connection, 2, |transaction| {
        Ok(transaction
            .execute_batch("INSERT INTO tracks(id,edition_id,metadata) VALUES(1,99,'{}');")?)
    })
    .unwrap_err();
    assert!(error.to_string().contains("version 2"));
    assert_eq!(version(&connection).unwrap(), 1);
    let tracks: i64 = connection
        .query_row("SELECT count(*) FROM tracks", [], |row| row.get(0))
        .unwrap();
    assert_eq!(tracks, 0);
    step(&mut connection, 2, |_| Ok(())).unwrap();
    assert_eq!(version(&connection).unwrap(), 2);
}

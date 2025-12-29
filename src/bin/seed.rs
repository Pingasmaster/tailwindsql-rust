#![allow(clippy::multiple_crate_versions)]

use std::path::PathBuf;

use tailwindsql::db::seed_database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("tailwindsql.db");
    let mut conn = rusqlite::Connection::open(&path)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    seed_database(&mut conn)?;
    Ok(())
}

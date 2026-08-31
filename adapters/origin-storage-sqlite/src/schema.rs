use origin_domain::{AppError, Result};
use rusqlite::Connection;

/// Schema version this build expects. Bump it together with a new migration below.
const CURRENT_VERSION: i64 = 1;

/// Bring the database up to [`CURRENT_VERSION`].
///
/// Migrations are forward-only and tracked in SQLite's own `user_version`, so no
/// bookkeeping table is needed.
pub(crate) fn apply(connection: &Connection) -> Result<()> {
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(map_error)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(map_error)?;

    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(map_error)?;

    if version > CURRENT_VERSION {
        return Err(AppError::storage(format!(
            "database schema version {version} is newer than this build supports \
             ({CURRENT_VERSION}) — the application was probably downgraded"
        )));
    }

    if version < 1 {
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS records (
                     namespace  TEXT NOT NULL,
                     key        TEXT NOT NULL,
                     value      TEXT NOT NULL,
                     stored_at  TEXT NOT NULL,
                     expires_at TEXT,
                     PRIMARY KEY (namespace, key)
                 ) WITHOUT ROWID;

                 CREATE INDEX IF NOT EXISTS records_expires_at
                     ON records (expires_at) WHERE expires_at IS NOT NULL;",
            )
            .map_err(map_error)?;
    }

    connection
        .pragma_update(None, "user_version", CURRENT_VERSION)
        .map_err(map_error)?;

    tracing::debug!(version = CURRENT_VERSION, "sqlite schema ready");
    Ok(())
}

fn map_error(error: rusqlite::Error) -> AppError {
    AppError::storage(error.to_string())
}

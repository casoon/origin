//! SQLite implementation of [`origin_storage::Storage`].
//!
//! SQLite holds cache, read models, sync metadata and settings — never credentials
//! (ADR-0008). Deleting the database file must cost the user nothing but a resync.
//!
//! `rusqlite` is blocking, so every call runs on the blocking pool rather than
//! stalling the async runtime.

mod schema;

use async_trait::async_trait;
use origin_domain::{AppError, Result};
use origin_storage::{Record, Storage, StorageKey};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone)]
pub struct SqliteStorage {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    /// Open (and create if needed) the database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path.as_ref()).map_err(to_storage_error)?;
        Self::from_connection(connection)
    }

    /// A private in-memory database. Useful for tests and for a `--no-persist` run.
    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory().map_err(to_storage_error)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        schema::apply(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Delete every record whose expiry has passed.
    ///
    /// Reads already ignore expired records; this only reclaims disk space, so it is
    /// safe to call on a schedule or never.
    pub async fn prune_expired(&self, now: OffsetDateTime) -> Result<usize> {
        let now = encode_time(now)?;
        self.with_connection(move |connection| {
            let removed = connection
                .execute(
                    "DELETE FROM records WHERE expires_at IS NOT NULL AND expires_at <= ?1",
                    params![now],
                )
                .map_err(to_storage_error)?;
            Ok(removed)
        })
        .await
    }

    /// Run a blocking database operation on the blocking pool.
    async fn with_connection<T, F>(&self, operation: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T> + Send + 'static,
    {
        let connection = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let guard = connection
                .lock()
                .map_err(|_| AppError::storage("sqlite connection poisoned"))?;
            operation(&guard)
        })
        .await
        .map_err(|error| AppError::storage(format!("storage task failed: {error}")))?
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn get(&self, key: &StorageKey) -> Result<Option<Record>> {
        let (namespace, name) = split(key);
        self.with_connection(move |connection| {
            connection
                .query_row(
                    "SELECT value, stored_at, expires_at FROM records \
                     WHERE namespace = ?1 AND key = ?2",
                    params![namespace, name],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(to_storage_error)?
                .map(|(value, stored_at, expires_at)| {
                    let mut record = Record::new(value, decode_time(&stored_at)?);
                    record.expires_at = expires_at.as_deref().map(decode_time).transpose()?;
                    Ok(record)
                })
                .transpose()
        })
        .await
    }

    async fn put(&self, key: &StorageKey, record: Record) -> Result<()> {
        let (namespace, name) = split(key);
        let stored_at = encode_time(record.stored_at)?;
        let expires_at = record.expires_at.map(encode_time).transpose()?;
        let value = record.value;

        self.with_connection(move |connection| {
            connection
                .execute(
                    "INSERT INTO records (namespace, key, value, stored_at, expires_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5) \
                     ON CONFLICT(namespace, key) DO UPDATE SET \
                       value = excluded.value, \
                       stored_at = excluded.stored_at, \
                       expires_at = excluded.expires_at",
                    params![namespace, name, value, stored_at, expires_at],
                )
                .map_err(to_storage_error)?;
            Ok(())
        })
        .await
    }

    async fn delete(&self, key: &StorageKey) -> Result<()> {
        let (namespace, name) = split(key);
        self.with_connection(move |connection| {
            connection
                .execute(
                    "DELETE FROM records WHERE namespace = ?1 AND key = ?2",
                    params![namespace, name],
                )
                .map_err(to_storage_error)?;
            Ok(())
        })
        .await
    }

    async fn keys(&self, namespace: &str) -> Result<Vec<StorageKey>> {
        let namespace = namespace.to_owned();
        self.with_connection(move |connection| {
            let mut statement = connection
                .prepare("SELECT key FROM records WHERE namespace = ?1")
                .map_err(to_storage_error)?;

            let keys = statement
                .query_map(params![namespace], |row| row.get::<_, String>(0))
                .map_err(to_storage_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(to_storage_error)?
                .into_iter()
                .map(|key| StorageKey::new(&namespace, key))
                .collect();

            Ok(keys)
        })
        .await
    }

    async fn clear(&self, namespace: &str) -> Result<()> {
        let namespace = namespace.to_owned();
        self.with_connection(move |connection| {
            connection
                .execute(
                    "DELETE FROM records WHERE namespace = ?1",
                    params![namespace],
                )
                .map_err(to_storage_error)?;
            Ok(())
        })
        .await
    }

    async fn clear_prefix(&self, prefix: &str) -> Result<usize> {
        // GLOB rather than LIKE: LIKE would treat `_` and `%` in a connector or
        // account id as wildcards, so `acct.gh_x.` would also delete `acct.ghax.`.
        let pattern = format!("{}*", glob_escape(prefix));

        self.with_connection(move |connection| {
            let removed = connection
                .execute(
                    "DELETE FROM records WHERE namespace GLOB ?1",
                    params![pattern],
                )
                .map_err(to_storage_error)?;
            Ok(removed)
        })
        .await
    }
}

/// Escape the characters GLOB treats specially, so a prefix is matched literally.
fn glob_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '*' | '?' | '[' | ']' => {
                escaped.push('[');
                escaped.push(character);
                escaped.push(']');
            }
            other => escaped.push(other),
        }
    }
    escaped
}

fn split(key: &StorageKey) -> (String, String) {
    (key.namespace().to_owned(), key.key().to_owned())
}

fn encode_time(value: OffsetDateTime) -> Result<String> {
    value
        .format(&Rfc3339)
        .map_err(|error| AppError::storage(format!("cannot format timestamp: {error}")))
}

fn decode_time(value: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| AppError::storage(format!("cannot parse timestamp {value:?}: {error}")))
}

/// The single place where a `rusqlite` error becomes an Origin error. Nothing
/// database-specific travels further up (ADR-0002).
fn to_storage_error(error: rusqlite::Error) -> AppError {
    AppError::storage(error.to_string())
}

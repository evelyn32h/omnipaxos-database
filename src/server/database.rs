use omnipaxos_kv::common::kv::{KVCommand, ReadConsistency};
use sqlx::SqlitePool;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Self {
        // Retrieve the database connection string from environment variables;
        // if not set, use a default value for SQLite
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:db.sqlite".to_string());
        let pool = SqlitePool::connect(&database_url)
            .await
            .expect("Failed to connect to SQLite database");

        // Create the 'kv' table if it does not exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT
            );"
        )
        .execute(&pool)
        .await
        .expect("Failed to create table in SQLite database");

        Self { pool }
    }

    pub async fn handle_command(&mut self, command: KVCommand) -> Option<String> {
        match command {
            // Handle the SQLQuery variant: if the SQL statement starts with SELECT, it is treated as a read operation;
            // otherwise, it is a write operation.
            KVCommand::SQLQuery { query, consistency } => {
                if query.trim_start().to_uppercase().starts_with("SELECT") {
                    match consistency {
                        Some(consistency_level) => match consistency_level {
                            ReadConsistency::Leader => {
                                // Leader read: perform a direct query using the primary database connection.
                                sqlx::query_scalar::<_, String>(&query)
                                    .fetch_optional(&self.pool)
                                    .await
                                    .ok()
                                    .flatten()
                            },
                            ReadConsistency::Local => {
                                // Local read: simulate a read from a nearby node.
                                sqlx::query_scalar::<_, String>(&query)
                                    .fetch_optional(&self.pool)
                                    .await
                                    .ok()
                                    .flatten()
                            },
                            ReadConsistency::Linearizable => {
                                // Linearizable read: execute the query within a transaction with exclusive locking
                                // to ensure stricter consistency.
                                let mut tx = self.pool.begin().await.ok()?;
                                
                                // Set exclusive lock mode in SQLite
                                sqlx::query("PRAGMA locking_mode = EXCLUSIVE")
                                    .execute(&mut tx)
                                    .await
                                    .ok()?;
                                
                                let res = sqlx::query_scalar::<_, String>(&query)
                                    .fetch_optional(&mut tx)
                                    .await
                                    .ok()
                                    .flatten();
                                
                                tx.commit().await.ok()?;
                                res
                            },
                        },
                        None => {
                            // If no consistency level is specified, default to using Leader read.
                            sqlx::query_scalar::<_, String>(&query)
                                .fetch_optional(&self.pool)
                                .await
                                .ok()
                                .flatten()
                        }
                    }
                } else {
                    // Write operation: execute the SQL statement directly.
                    sqlx::query(&query)
                        .execute(&self.pool)
                        .await
                        .ok();
                    None
                }
            }
            // Handle the legacy Put command
            KVCommand::Put(key, value) => {
                let upsert = "INSERT INTO kv (key, value) VALUES (?, ?)
                              ON CONFLICT (key) DO UPDATE SET value = excluded.value";
                sqlx::query(upsert)
                    .bind(&key)
                    .bind(&value)
                    .execute(&self.pool)
                    .await
                    .ok();
                None
            }
            // Handle the legacy Delete command
            KVCommand::Delete(key) => {
                let del = "DELETE FROM kv WHERE key = ?";
                sqlx::query(del)
                    .bind(&key)
                    .execute(&self.pool)
                    .await
                    .ok();
                None
            }
            // Handle the legacy Get command
            KVCommand::Get(key) => {
                let sel = "SELECT value FROM kv WHERE key = ?";
                sqlx::query_scalar::<_, String>(sel)
                    .bind(&key)
                    .fetch_optional(&self.pool)
                    .await
                    .ok()
                    .flatten()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnipaxos_kv::common::kv::{KVCommand, ReadConsistency};

    // Use an in-memory SQLite database for testing
    const TEST_DB_URL: &str = "sqlite::memory:";

    #[tokio::test]
    async fn test_sql_write_then_read() {
        std::env::set_var("DATABASE_URL", TEST_DB_URL);

        let mut db = Database::new().await;

        // Write operation: insert key 'test_key' with value 'test_value'
        let write_query = "INSERT INTO kv (key, value) VALUES ('test_key', 'test_value')";
        let write_cmd = KVCommand::SQLQuery {
            query: write_query.to_string(),
            consistency: None,
        };
        db.handle_command(write_cmd).await;

        // Read operation: retrieve the value for key 'test_key'
        let read_query = "SELECT value FROM kv WHERE key = 'test_key'";
        let read_cmd = KVCommand::SQLQuery {
            query: read_query.to_string(),
            consistency: Some(ReadConsistency::Local),
        };
        let result = db.handle_command(read_cmd).await;

        assert_eq!(result, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_read_consistency_leader() {
        std::env::set_var("DATABASE_URL", TEST_DB_URL);
        let mut db = Database::new().await;

        // Write operation: insert key 'leader_key' with value 'leader_value'
        let write_query = "INSERT INTO kv (key, value) VALUES ('leader_key', 'leader_value')";
        let write_cmd = KVCommand::SQLQuery {
            query: write_query.to_string(),
            consistency: None,
        };
        db.handle_command(write_cmd).await;

        // Read operation using Leader consistency
        let read_query = "SELECT value FROM kv WHERE key = 'leader_key'";
        let read_cmd = KVCommand::SQLQuery {
            query: read_query.to_string(),
            consistency: Some(ReadConsistency::Leader),
        };
        let result = db.handle_command(read_cmd).await;
        assert_eq!(result, Some("leader_value".to_string()));
    }

    #[tokio::test]
    async fn test_read_consistency_local() {
        std::env::set_var("DATABASE_URL", TEST_DB_URL);
        let mut db = Database::new().await;

        // Write operation: insert key 'local_key' with value 'local_value'
        let write_query = "INSERT INTO kv (key, value) VALUES ('local_key', 'local_value')";
        let write_cmd = KVCommand::SQLQuery {
            query: write_query.to_string(),
            consistency: None,
        };
        db.handle_command(write_cmd).await;

        // Read operation using Local consistency
        let read_query = "SELECT value FROM kv WHERE key = 'local_key'";
        let read_cmd = KVCommand::SQLQuery {
            query: read_query.to_string(),
            consistency: Some(ReadConsistency::Local),
        };
        let result = db.handle_command(read_cmd).await;
        assert_eq!(result, Some("local_value".to_string()));
    }

    #[tokio::test]
    async fn test_read_consistency_linearizable() {
        std::env::set_var("DATABASE_URL", TEST_DB_URL);
        let mut db = Database::new().await;

        // Write operation: insert key 'linear_key' with value 'linear_value'
        let write_query = "INSERT INTO kv (key, value) VALUES ('linear_key', 'linear_value')";
        let write_cmd = KVCommand::SQLQuery {
            query: write_query.to_string(),
            consistency: None,
        };
        db.handle_command(write_cmd).await;

        // Read operation using Linearizable consistency
        let read_query = "SELECT value FROM kv WHERE key = 'linear_key'";
        let read_cmd = KVCommand::SQLQuery {
            query: read_query.to_string(),
            consistency: Some(ReadConsistency::Linearizable),
        };
        let result = db.handle_command(read_cmd).await;
        assert_eq!(result, Some("linear_value".to_string()));
    }
}





/* 
use omnipaxos_kv::common::kv::KVCommand;
use std::collections::HashMap;

pub struct Database {
    db: HashMap<String, String>,
}

impl Database {
    pub fn new() -> Self {
        Self { db: HashMap::new() }
    }

    pub fn handle_command(&mut self, command: KVCommand) -> Option<Option<String>> {
        match command {
            KVCommand::Put(key, value) => {
                self.db.insert(key, value);
                None
            }
            KVCommand::Delete(key) => {
                self.db.remove(&key);
                None
            }
            KVCommand::Get(key) => Some(self.db.get(&key).map(|v| v.clone())),
        }
    }
}
*/
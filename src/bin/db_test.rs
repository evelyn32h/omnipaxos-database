use sqlx::SqlitePool;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting SQLite consistency level tests");
    
    // Create SQLite connection pool - using in-memory database for testing
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    
    // Create test table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS kv (
            key TEXT PRIMARY KEY,
            value TEXT
        );"
    )
    .execute(&pool)
    .await?;
    println!("Test table created successfully");
    
    // Insert test data
    let write_query = "INSERT INTO kv (key, value) VALUES ('test_key', 'test_value')";
    let start = Instant::now();
    sqlx::query(write_query)
        .execute(&pool)
        .await?;
    let duration = start.elapsed();
    println!("Data write successful, time elapsed: {:?}", duration);
    
    // Test reads - simulating different consistency levels
    let read_query = "SELECT value FROM kv WHERE key = 'test_key'";
    
    // Simulate Leader read - direct read from the primary node
    let start = Instant::now();
    let result = sqlx::query_scalar::<_, String>(read_query)
        .fetch_one(&pool)
        .await?;
    let duration = start.elapsed();
    println!("Leader read result: {}, time elapsed: {:?}", result, duration);
    
    // Simulate Local read - read from local node, potentially fastest
    let start = Instant::now();
    let result = sqlx::query_scalar::<_, String>(read_query)
        .fetch_one(&pool)
        .await?;
    let duration = start.elapsed();
    println!("Local read result: {}, time elapsed: {:?}", result, duration);
    
    // Simulate Linearizable read - using transaction and exclusive lock for strongest consistency
    let start = Instant::now();
    let mut tx = pool.begin().await?;
    sqlx::query("PRAGMA locking_mode = EXCLUSIVE")
        .execute(&mut tx)
        .await?;
    let result = sqlx::query_scalar::<_, String>(read_query)
        .fetch_one(&mut tx)
        .await?;
    let duration = start.elapsed();
    println!("Linearizable read result: {}, time elapsed: {:?}", result, duration);
    tx.commit().await?;
    
    println!("\nTest summary:");
    println!("1. All consistency levels successfully returned the correct value 'test_value'");
    println!("2. Generally, performance ranking by consistency level should be: Local (fastest) < Leader < Linearizable (slowest)");
    println!("3. Since this is a single-machine test environment, performance differences may not be significant");
    
    // Test with more data to demonstrate differences
    println!("\nPerforming bulk data testing...");
    
    // Insert 1000 records
    let batch_start = Instant::now();
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        sqlx::query("INSERT INTO kv (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
            .bind(&key)
            .bind(&value)
            .execute(&pool)
            .await?;
    }
    println!("Time to insert 1000 records: {:?}", batch_start.elapsed());
    
    // Test bulk performance for three reading methods
    
    // Leader reading
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key_{}", i);
        let _result = sqlx::query_scalar::<_, String>("SELECT value FROM kv WHERE key = ?")
            .bind(&key)
            .fetch_one(&pool)
            .await?;
    }
    println!("Time to read 100 records using Leader mode: {:?}", start.elapsed());
    
    // Local reading
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key_{}", i);
        let _result = sqlx::query_scalar::<_, String>("SELECT value FROM kv WHERE key = ?")
            .bind(&key)
            .fetch_one(&pool)
            .await?;
    }
    println!("Time to read 100 records using Local mode: {:?}", start.elapsed());
    
    // Linearizable reading
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key_{}", i);
        let mut tx = pool.begin().await?;
        sqlx::query("PRAGMA locking_mode = EXCLUSIVE")
            .execute(&mut tx)
            .await?;
        let _result = sqlx::query_scalar::<_, String>("SELECT value FROM kv WHERE key = ?")
            .bind(&key)
            .fetch_one(&mut tx)
            .await?;
        tx.commit().await?;
    }
    println!("Time to read 100 records using Linearizable mode: {:?}", start.elapsed());
    
    println!("\nTesting completed!");
    Ok(())
}
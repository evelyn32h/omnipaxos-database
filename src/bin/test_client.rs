// src/bin/test_client.rs
use futures::{SinkExt, StreamExt};
use omnipaxos_kv::common::{
    kv::{KVCommand, ReadConsistency},
    messages::*,
    utils::*,
};
use tokio::net::TcpStream;
use std::time::Duration;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("Connecting to server...");
    
    // Connect to server 1
    let stream = match TcpStream::connect("127.0.0.1:8001").await {
        Ok(stream) => {
            println!("Connected to server");
            stream
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
            return;
        }
    };
    
    let mut conn = frame_registration_connection(stream);
    if let Err(e) = conn.send(RegistrationMessage::ClientRegister).await {
        println!("Failed to send registration: {}", e);
        return;
    }
    println!("Registered as client");
    
    let stream = conn.into_inner().into_inner();
    let (mut from_server, mut to_server) = frame_clients_connection(stream);
    
    // Insert test data
    let command_id = 1;
    let write_query = "INSERT INTO kv (key, value) VALUES ('test_key', 'test_value') ON CONFLICT(key) DO UPDATE SET value=excluded.value";
    let write_cmd = KVCommand::SQLQuery {
        query: write_query.to_string(),
        consistency: None,
    };
    
    println!("Sending write command: {}", write_query);
    let start = Instant::now();
    if let Err(e) = to_server.send(ClientMessage::Append(command_id, write_cmd)).await {
        println!("Failed to send write command: {}", e);
        return;
    }
    
    // Wait for response
    match from_server.next().await {
        Some(Ok(response)) => {
            let duration = start.elapsed();
            println!("Write response: {:?}", response);
            println!("Write operation took: {:?}", duration);
        },
        Some(Err(e)) => println!("Error receiving response: {:?}", e),
        None => println!("No response received"),
    }
    
    // Wait a bit for replication
    println!("Waiting for replication...");
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Test leader read
    let command_id = 2;
    let read_query = "SELECT value FROM kv WHERE key = 'test_key'";
    let leader_read = KVCommand::SQLQuery {
        query: read_query.to_string(),
        consistency: Some(ReadConsistency::Leader),
    };
    
    println!("Sending leader read command");
    let start = Instant::now();
    if let Err(e) = to_server.send(ClientMessage::Append(command_id, leader_read)).await {
        println!("Failed to send leader read command: {}", e);
        return;
    }
    
    // Wait for response
    match from_server.next().await {
        Some(Ok(response)) => {
            let duration = start.elapsed();
            println!("Leader read response: {:?}", response);
            println!("Leader read operation took: {:?}", duration);
        },
        Some(Err(e)) => println!("Error receiving response: {:?}", e),
        None => println!("No response received"),
    }
    
    // Test local read
    let command_id = 3;
    let local_read = KVCommand::SQLQuery {
        query: read_query.to_string(),
        consistency: Some(ReadConsistency::Local),
    };
    
    println!("Sending local read command");
    let start = Instant::now();
    if let Err(e) = to_server.send(ClientMessage::Append(command_id, local_read)).await {
        println!("Failed to send local read command: {}", e);
        return;
    }
    
    // Wait for response
    match from_server.next().await {
        Some(Ok(response)) => {
            let duration = start.elapsed();
            println!("Local read response: {:?}", response);
            println!("Local read operation took: {:?}", duration);
        },
        Some(Err(e)) => println!("Error receiving response: {:?}", e),
        None => println!("No response received"),
    }
    
    // Test linearizable read
    let command_id = 4;
    let linearizable_read = KVCommand::SQLQuery {
        query: read_query.to_string(),
        consistency: Some(ReadConsistency::Linearizable),
    };
    
    println!("Sending linearizable read command");
    let start = Instant::now();
    if let Err(e) = to_server.send(ClientMessage::Append(command_id, linearizable_read)).await {
        println!("Failed to send linearizable read command: {}", e);
        return;
    }
    
    // Wait for response
    match from_server.next().await {
        Some(Ok(response)) => {
            let duration = start.elapsed();
            println!("Linearizable read response: {:?}", response);
            println!("Linearizable read operation took: {:?}", duration);
        },
        Some(Err(e)) => println!("Error receiving response: {:?}", e),
        None => println!("No response received"),
    }
    
    println!("Test completed successfully");
}
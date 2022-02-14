use anyhow::Result;
use author_console::streams::{BackupLoop, ChannelAuthor, MessageRetriever};
use author_console::http::api_server;

use std::sync::{Arc};
use parking_lot::Mutex;
use std::fs::File;
use mysql::prelude::Queryable;
use rand::Rng;

#[tokio::main]
async fn main() -> Result<()> {
    let config: serde_json::Value = serde_json::from_reader(File::open("config.json").unwrap()).unwrap();
    let seed: String;
    if config["seed"].is_null() {
        let alph9 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
        seed = (0..10)
            .map(|_| alph9.chars().nth(rand::thread_rng().gen_range(0, 27)).unwrap())
            .collect::<String>();
    } else {
        seed = config["seed"].as_str().unwrap().to_string()
    }

    let node = config["node"].as_str().unwrap();
    let port = config["api_port"].as_u64().unwrap() as u16;
    let psk = config["pre_shared_key"].as_str().unwrap();

    // SQL options
    let sql_opts = mysql::OptsBuilder::new()
        .user(Some("root"))
        .pass(Some("root"))
        .db_name(Some("molina"));

    print!("Starting SQL connection... ");
    let pool = mysql::Pool::new(sql_opts)?;
    let mut conn = pool.get_conn()?;
    conn.query_drop(r"CREATE TABLE IF NOT EXISTS molina.readings (
        sensor_id text not null,
        reading_id varchar(64) not null,
        reading text not null
    )")?;
    conn.query_drop(r"CREATE TABLE IF NOT EXISTS molina.sheet_readings (
        sheet_id text not null,
        reading_id varchar(64) not null,
        reading text not null
    )")?;
    conn.query_drop(r"CREATE TABLE IF NOT EXISTS molina.annotations (
        reading_id varchar(64) not null,
        annotation text not null
    )")?;
    println!("Done.");

    let author = Arc::new(Mutex::new(ChannelAuthor::new(seed.as_str(), node, psk).await.unwrap()));
    let channel_address = author.lock().get_announcement_id().unwrap();

    println!("\nChannel Address - {}:{}\n", channel_address.0, channel_address.1);
    let retriever = MessageRetriever::new(author.clone(), pool.clone());
    let backup = BackupLoop::new(author.clone());
    let _retriever = MessageRetriever::start(retriever).unwrap();
    let _backup = BackupLoop::start(backup).unwrap();

    match api_server::start(port, author, pool).await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e))
    }
}

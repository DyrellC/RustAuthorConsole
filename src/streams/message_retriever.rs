use anyhow::Result;
use crate::streams::ChannelAuthor;
use std::{
    sync::Arc,
    time::Duration
};
use futures::executor::block_on;
use mysql::params;
use mysql::prelude::Queryable;
use parking_lot::Mutex;

pub struct MessageRetriever {
    author: Arc<Mutex<ChannelAuthor>>,
    sql: mysql::Pool,
}

unsafe impl Send for MessageRetriever {}

impl MessageRetriever {
    pub fn new(
        author: Arc<Mutex<ChannelAuthor>>,
        sql: mysql::Pool,
    ) -> Self {
        MessageRetriever {
            author,
            sql
        }
    }

    pub fn start(retriever: Self) -> Result<()> {
        println!("Retrieval thread spawning. Searching for new messages...");
        tokio::task::spawn(async move {
            loop {
                Self::handle_messages(&retriever).await;
                tokio::time::sleep(Duration::from_millis(3000)).await;
            }
        });
        Ok(())
    }

    async fn handle_messages(&self) {
        //TODO: Handle all panics here
        let mut author = self.author.lock();
        let msgs = tokio::task::block_in_place(|| {block_on(author.get_next_msgs())}).unwrap();
        let mut conn = self.sql.get_conn().unwrap();

        for reading in msgs.readings {
            println!("Got a new reading message");
            let sensor_id = reading.get_sensor_id().clone();

            let reading_id = sha256::digest(&serde_json::to_string(&reading).unwrap());
            conn.exec_drop(r"INSERT INTO molina.readings (sensor_id, reading_id, reading)
                VALUES (:sensor_id, :reading_id, :reading)", params! {
                    "sensor_id" => sensor_id.0.clone(),
                    "reading_id" => reading_id,
                    "reading" => serde_json::to_string(&reading).unwrap()
                }).unwrap();

            println!("Stored reading\n");
        }

        for sheet_reading in msgs.sheet_readings {
            println!("Got a new sheet reading message");
            let sheet_id = sheet_reading.sheet_id.clone();

            let reading_id = sha256::digest(&serde_json::to_string(&sheet_reading).unwrap());
            conn.exec_drop(r"INSERT INTO molina.sheet_readings (sheet_id, reading_id, reading)
                VALUES (:sheet_id, :reading_id, :reading)", params! {
                    "sheet_id" => sheet_id.0.clone(),
                    "reading_id" => reading_id,
                    "reading" => sheet_reading.get_value().unwrap().0
                }).unwrap();


            println!("Stored sheet reading\n");
        }

        for annotation in msgs.annotations {
            let ann_type = annotation.action.clone();
            for annotation in annotation.get_annotations().items {
                println!("Storing {} annotation: {}", ann_type, serde_json::to_string(&annotation).unwrap());
                let reading_id = annotation.get_reading_id().clone();

                conn.exec_drop(r"INSERT INTO molina.annotations (reading_id, annotation)
                VALUES (:reading_id, :annotation)", params! {
                    "reading_id" => &reading_id.0,
                    "annotation" => serde_json::to_string(&annotation).unwrap()
                }).unwrap();

                println!("Stored {}\n", reading_id.0);
            }
        }

        for other in msgs.other {
            let json = serde_json::from_str::<serde_json::Value>(&other).unwrap();
            println!("Got us a message: {:#}\n", json);
            println!("Message not stored");
        }
    }

}
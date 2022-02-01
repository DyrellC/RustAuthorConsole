use anyhow::Result;
use crate::streams::ChannelAuthor;
use crate::store::{AnnotationStore, ReadingStore};
use std::{
    sync::Arc,
    time::Duration
};
use iota_streams::app::futures::executor::block_on;
use parking_lot::Mutex;

pub struct MessageRetriever {
    author: Arc<Mutex<ChannelAuthor>>,
    annotation_store: Arc<Mutex<AnnotationStore>>,
    reading_store: Arc<Mutex<ReadingStore>>,
}

unsafe impl Send for MessageRetriever {}

impl MessageRetriever {
    pub fn new(
        author: Arc<Mutex<ChannelAuthor>>,
        annotation_store: Arc<Mutex<AnnotationStore>>,
        reading_store: Arc<Mutex<ReadingStore>>,
    ) -> Self {
        MessageRetriever {
            author,
            annotation_store,
            reading_store
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

        for reading in msgs.readings {
            println!("Got a new reading message");
            let sensor_id = reading.get_sensor_id().clone();
            println!("Storing reading: {}", serde_json::to_string(&reading).unwrap());
            let mut reading_store = self.reading_store.lock();
            reading_store.insert_reading(&sensor_id, reading).unwrap();
            println!("Stored reading\n");
        }

        for sheet_reading in msgs.sheet_readings {
            println!("Got a new sheet reading message");
            let sheet_id = sheet_reading.sheet_id.clone();
            println!("Storing sheet reading: {}", sheet_reading.get_value().unwrap().0);
            let mut reading_store = self.reading_store.lock();
            reading_store.insert_sheet_reading(&sheet_id, sheet_reading).unwrap();
            println!("Stored sheet reading\n");
        }

        for annotation in msgs.annotations {
            let ann_type = annotation.action.clone();
            let mut annotation_store = self.annotation_store.lock();
            for annotation in annotation.get_annotations().items {
                println!("Storing {} annotation: {}", ann_type, serde_json::to_string(&annotation).unwrap());
                let reading_id = annotation.get_reading_id().clone();
                annotation_store.insert(&reading_id, annotation).unwrap();
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
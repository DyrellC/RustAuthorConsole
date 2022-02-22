use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::streams::ChannelAuthor;
use anyhow::Result;
use futures::executor::block_on;

pub struct BackupLoop {
    author: Arc<Mutex<ChannelAuthor>>,
}

impl BackupLoop {
    pub fn new(
        author: Arc<Mutex<ChannelAuthor>>,
    ) -> Self {
        BackupLoop {
            author
        }
    }

    pub fn start(backup_loop: Self) -> Result<()> {
        println!("Retrieval thread spawning. Searching for new messages...");
        tokio::task::spawn(async move {
            loop {
                Self::backup(&backup_loop).await;
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        });
        Ok(())
    }


    async fn backup(&self) {
        let author = self.author.lock().await;
        tokio::task::block_in_place(|| {block_on(author.backup())}).unwrap();
    }
}
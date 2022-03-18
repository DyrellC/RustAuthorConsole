use std::fs::File;
use std::io::{Read, Write};
use anyhow::{Result, anyhow};
use iota_streams::app::transport::{
    tangle::{
        client::{Client},
        MsgId,
    }
};
use iota_streams::app_channels::api::{pskid_from_psk, psk_from_seed, PskId, tangle::{Address, Author, ChannelAddress, MessageContent}, ChannelType};
use iota_streams::core_edsig::signature::ed25519::PublicKey;

use std::str::FromStr;
use futures::TryStreamExt;
use crate::models::{Reading, Annotation, SheetReading};

pub struct ChannelAuthor {
    author: Author<Client>,
    announcement_id: Address,
    channel_address: ChannelAddress,
    subscriber_key_id: PskId,
}

#[derive(Default)]
pub struct MessageReturn {
    pub readings: Vec<Reading>,
    pub sheet_readings: Vec<SheetReading>,
    pub annotations: Vec<Annotation>,
    pub other: Vec<String>
}

impl ChannelAuthor {
    pub async fn new(seed: &str, node: &str, psk_str: &str) -> Result<ChannelAuthor> {
        // Create Client instance
        let client = Client::new_from_url(node);
        let psk = psk_from_seed(psk_str.as_bytes());
        let pskid = pskid_from_psk(&psk);

        match std::fs::File::open("author_backup.bin") {
            Ok(mut backup) => {
                println!("Creating author from backup...");
                let mut bytes = Vec::new();
                backup.read_to_end(&mut bytes)?;
                let author = Author::import(&bytes, "password", client).await?;
                let announcement_id = author.announcement_link().unwrap();
                let channel_address = announcement_id.appinst.clone();
                println!("Announcement id: {}", announcement_id.to_msg_index());

                Ok(ChannelAuthor { author, announcement_id, channel_address, subscriber_key_id: pskid })
            }
            Err(_) => {
                // Generate a multi branch Author instance and start the channel
                let mut author = Author::new(seed, ChannelType::MultiBranch, client);
                let announcement_id = author.send_announce().await?;
                let channel_address = announcement_id.appinst.clone();
                println!("Announcement id: {}", announcement_id.to_msg_index());

                author.store_psk(pskid, psk)?;

                println!("Making backup and storing in \"author_backup.bin\"");
                let backup = author.export("password").await?;
                println!("Backup length: {}", backup.len());
                let mut file = File::create("author_backup.bin")?;
                file.write(backup.as_slice())?;

                Ok(ChannelAuthor { author, announcement_id, channel_address, subscriber_key_id: pskid })
            }
        }
    }

    pub async fn backup(&self) -> Result<()> {
        println!("Making backup and storing in \"author_backup.bin\"");
        let backup = self.author.export("password").await?;
        let mut file = File::create("author_backup.bin")?;
        file.write(backup.as_slice())?;
        println!("Backed up...");
        Ok(())
    }

    pub fn get_channel_address(&self) -> Result<String> {
        let channel_address = &self.channel_address.to_string();
        Ok(String::from_str(channel_address).unwrap())
    }

    pub fn get_announcement_id(&self) -> Result<(String, String)> {
        let appinst = &self.announcement_id.appinst.to_string();
        let msgid = &self.announcement_id.msgid.to_string();
        Ok((String::from_str(appinst).unwrap(), String::from_str(msgid).unwrap()))
    }

    pub async fn subscribe(&mut self, link: &str, pk: &Vec<u8>) -> Result<Address> {
        match MsgId::from_str(link) {
            Ok(msgid) => {
                println!("receiving subscription");
                let address = Address {
                    appinst: self.channel_address.clone(),
                    msgid
                };
                self.author.receive_subscribe(&address).await?;
                println!("Sending keyload");
                let identifiers = vec![self.subscriber_key_id.into(), PublicKey::from_bytes(pk).unwrap().into()];
                let keyload = self.author.send_keyload(&self.announcement_id, &identifiers).await?;

                // Return the sequence message link
                Ok(keyload.1.unwrap())
            },
            Err(_) => {
                Err(anyhow!("Error getting msgid from provided link: {}", link))
            }
        }
    }

    pub async fn get_next_msgs(&mut self) -> Result<MessageReturn> {
        let mut found_msgs = MessageReturn::default();

        let mut msgs = self.author.messages();
        while let Some(msg) = msgs.try_next().await? {
            match msg.body {
                MessageContent::SignedPacket {pk: _, public_payload: _, masked_payload: m} => {
                    print!("Recieved a signed packet: ");
                    let reading: serde_json::Result<Reading> = serde_json::from_slice(&m.0);
                    match reading {
                        Ok(r) => {
                            println!("Got reading: {}", r.sensor_id.0);
                            found_msgs.readings.push(r)
                        },
                        Err(_) => {
                            let sheet_reading: serde_json::Result<SheetReading> = serde_json::from_slice(&m.0);
                            match sheet_reading {
                                Ok(sr) => {
                                    println!("Got sheet reading: {}", sr.sheet_id.0);
                                    found_msgs.sheet_readings.push(sr)
                                },
                                Err(_) => {
                                    let annotation: serde_json::Result<Annotation> = serde_json::from_slice(&m.0);
                                    match annotation {
                                        Ok(a) => {
                                            println!("Got annotation: {}", a.action);
                                            found_msgs.annotations.push(a)
                                        },
                                        Err(_) => {
                                            match String::from_utf8(m.0) {
                                                Ok(o) => {
                                                    println!("Got other: {}", o);
                                                    found_msgs.other.push(o)
                                                },
                                                Err(_) => println!("Error deserializing message")
                                            }
                                        }
                                    };
                                }
                            }
                        }
                    }
                }
                MessageContent::Unreadable(_) => println!("Message is unreadable"),
                MessageContent::Sequence => println!("Message is a sequence"),
                MessageContent::Keyload => println!("Message is a keyload"),
                _ => println!("Message type not supported")
            }
        }

        Ok(found_msgs)
    }
}

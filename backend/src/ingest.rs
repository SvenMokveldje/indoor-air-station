use std::io::Bytes;

use rumqttc::{AsyncClient, Event, EventLoop, Incoming, QoS};
use shared::Reading;
use tokio::task::JoinHandle;

use crate::db::Database;

pub struct MQTTClient {
    client: AsyncClient,
    eventloop: EventLoop,
    db: Database
}

impl MQTTClient {
    pub fn new(client: AsyncClient, eventloop: EventLoop, db: Database) -> Self {
        Self { client, eventloop, db }
    }

    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(self.run())
    }

    async fn run(mut self) {
        self.client
            .subscribe("weather/#", QoS::AtLeastOnce)
            .await
            .unwrap();
        
        loop {
            let event = self.eventloop.poll().await;
            match &event {
                Ok(notif) => {
                    println!("Event = {notif:?}");
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    break;
                }
            }

            if let Ok(Event::Incoming(Incoming::Publish(packet))) = event {
                let packet_payload = packet.payload.as_ref();
                
                match Reading::try_from(packet_payload) {
                    Ok(reading) => {
                        if let Err(err) = self.db.insert_reading(&reading).await {
                            eprintln!("Failed to store reading {}: {err}", reading.seq);
                        } 
                    }
                    Err(e) => eprintln!("{:?}", e),
                }
                
            }
        }
    }
}

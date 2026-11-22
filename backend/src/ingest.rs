use rumqttc::{AsyncClient, EventLoop, QoS};
use tokio::task::JoinHandle;

pub struct MQTTClient {
    client: AsyncClient,
    eventloop: EventLoop,
}

impl MQTTClient {
    pub fn new(client: AsyncClient, eventloop: EventLoop) -> Self {
        Self { client, eventloop }
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
                Ok(v) => {
                    println!("{:?}", v);
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    break;
                }
            }
        }
    }
}

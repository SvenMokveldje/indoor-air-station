#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::net::Ipv4Addr;
use core::num::NonZero;

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Runner, StackResources, Stack};
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::rng::Rng;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config as WifiConfig, Interface, Password, Ssid, WifiController,
};
use firmware::am2320::{self, Am2320};
use shared::Reading;


#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;
use esp_alloc as _;
use rust_mqtt::Bytes;
use rust_mqtt::buffer::BumpBuffer;
use rust_mqtt::client::Client;
use rust_mqtt::client::options::{ConnectOptions, PublicationOptions, TopicReference};
use rust_mqtt::config::KeepAlive;
use rust_mqtt::types::{MqttString, TopicName};
use static_cell::StaticCell;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 2.0.0
    // generator parameters: -o esp32c3 -o unstable-hal -o alloc -o embassy -o wifi -o stack-smashing-protection

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let wifi_controller =
        esp_radio::wifi::WifiController::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let wifi_interface = esp_radio::wifi::Interface::station();

    const SSID: &str = env!("SSID");
    const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
    
    let ssid: Ssid = SSID.try_into().expect("SSID longer than 32 bytes");
    let password: Password = WIFI_PASSWORD
        .try_into()
        .expect("password longer than 64 bytes");

    let config = embassy_net::Config::dhcpv4(Default::default());
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let resources = STACK_RESOURCES.init(StackResources::<3>::new());
    let (stack, runner) = embassy_net::new(wifi_interface, config, resources, seed);

    spawner.spawn(
        connection(wifi_controller, ssid, password).expect("Failed to spawn connection task"),
    );
    spawner.spawn(net_task(runner).expect("Failed to spawn network task"));

    stack.wait_link_up().await;

    loop {
        if let Some(config) = stack.config_v4() {
            esp_println::println!("Got IP: {}", config.address);
            break;
        }
        embassy_time::Timer::after(Duration::from_millis(500)).await;
    }

    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));

    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO0)
        .with_scl(peripherals.GPIO1);

    let am2320 = Am2320::new(i2c);

    spawner.spawn(read_sensor(stack, am2320).expect("Failed to spawn read_sensor task"));

    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(60)).await;
    }
}

#[embassy_executor::task]
pub async fn read_sensor(stack: Stack<'static>, mut am2320: Am2320<'static>) {
    const BROKER_IP: &str = env!("BROKER_IP");
    let adress: Ipv4Addr = BROKER_IP.parse().expect("BROKER_IP is not a vlid ipv4");
    const BROKER_PORT: u16 = 1883;

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    let remote_endpoint = (adress, BROKER_PORT);
    esp_println::println!("Connecting to broker and port at {}:{}", BROKER_IP, BROKER_PORT);
    let connection = socket.connect(remote_endpoint).await;
    if let Err(e) = connection {
        esp_println::println!("Failed to connect to broker: {:?}", e);
        return;
    }
    esp_println::println!("Connected to broker");

    let mut mqtt_buffer_storage = [0; 1024];
    let mut mqtt_buffer = BumpBuffer::new(&mut mqtt_buffer_storage);
    let mut client = Client::<_, _, 1, 1, 1, 1, 1, 0, 0>::new(&mut mqtt_buffer);
    let connect_options = ConnectOptions::new()
        .clean_start()
        .keep_alive(KeepAlive::Seconds(
            NonZero::new(60).expect("non-zero keep alive"),
        ));
    let client_id = MqttString::from_str("esp32c3").expect("valid MQTT client id");
    
    if let Err(e) = client
        .connect(socket, &connect_options, Some(client_id))
        .await
    {
        esp_println::println!("MQTT connect error: {:?}", e);
        return;
    }

    let topic = TopicName::new(
        MqttString::from_str("weather/reading").expect("valid MQTT topic string"),
    )
    .expect("valid MQTT topic name");
    let publish_options = PublicationOptions::new(TopicReference::Name(topic)).retain();
    
    
    loop {
        if !stack.is_link_up() || !stack.is_config_up() {
            esp_println::println!("failed to connect to network");
            break;
        }
        
        let measurement = match am2320.read().await {
            Ok(m) => m,
            Err(e) => {
                esp_println::println!("AM2320 read error: {:?}", e);
                continue;
            }
        };

        let reading: Reading = Reading {
            seq: 1,
            temperature_c: measurement.temperature_x10 as f32 / 10.0,
            humidity_pct: measurement.humidity_x10 as f32 / 10.0,
            pressure_hpa: None,
            co2_ppm: None,
        };
        
        esp_println::println!("{:?}", reading);

        let payload = match reading.to_json() {
            Ok(p) => p,
            Err(e) => {
                esp_println::println!("JSON serialization error: {:?}", e);
                continue;
            }
        };
        
        if let Err(e) = client.publish(&publish_options, payload.as_slice().into()).await {
            esp_println::println!("MQTT publish error: {:?}", e);
        }
        Timer::after(Duration::from_millis(3000)).await;
    }
}

#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>, ssid: Ssid, password: Password) {
    loop {
        if controller.is_connected() {
            let _ = controller.wait_for_disconnect_async().await;
            Timer::after(Duration::from_millis(5000)).await;
            continue;
        }

        let station_config = WifiConfig::Station(
            StationConfig::default()
                .with_ssid(ssid)
                .with_authentication(AuthenticationMethodConfig::Wpa2Personal(password)),
        );
        controller
            .set_config(&station_config)
            .expect("Failed to set WiFi configuration");

        esp_println::println!("About to connect...");
        match controller.connect_async().await {
            Ok(_) => esp_println::println!("Wifi connected!"),
            Err(e) => {
                esp_println::println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

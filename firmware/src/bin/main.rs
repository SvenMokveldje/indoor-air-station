#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::i2c;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::time::Rate;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

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

    let _wifi_controller =
        esp_radio::wifi::WifiController::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let _wifi_interface = esp_radio::wifi::Interface::station();

    // TODO: Spawn some tasks
    let _ = spawner;

    const AM2320_ADDR: u8 = 0x5C; // 7-bit address (0xB8 >> 1)

    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));

    let mut i2c = I2c::new(
        peripherals.I2C0,
        i2c_config,
    ).unwrap()
    .with_sda(peripherals.GPIO0)
    .with_scl(peripherals.GPIO1);

    loop {
        // Wake up 
        let _ = i2c.write(AM2320_ADDR, &[]);
        
        Timer::after(Duration::from_millis(1)).await;

        if let Err(e) = i2c.write(AM2320_ADDR, &[0x03, 0x00, 0x04]) {
            esp_println::println!("Command Error: {:?}", e);
        }

        Timer::after(Duration::from_millis(2)).await;
        
        let mut buf = [0u8; 8];
        if let Err(e) = i2c.read(AM2320_ADDR, &mut buf) {
            esp_println::println!("Read Error: {:?}", e);
        }

        let hum_high = buf[2] as u16;
        let hum_low = buf[3] as u16;

        let temp_high = buf[4] as u16;
        let temp_low = buf[5] as u16;

        let hum = ((hum_high * 256 + hum_low) as f32)/10.0;
        let temp = ((temp_high * 256 + temp_low) as f32)/10.0;
        
        esp_println::println!("hum: {}, temp: {}", hum, temp);

        Timer::after(Duration::from_millis(3000)).await;
    }
}

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
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use firmware::am2320::{Am2320};

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

    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));

    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO0)
        .with_scl(peripherals.GPIO1);
    
    let mut am2320 = Am2320::new(i2c);

    loop {
        let measurement = am2320.read().await;
        esp_println::println!("{:?}", measurement);
        

        Timer::after(Duration::from_millis(3000)).await;
    }
}

// use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

// fn main() {
//     // It is necessary to call this function once. Otherwise some patches to the runtime
//     // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
//     esp_idf_sys::link_patches();
//     println!("Hello, world!");
// }


// Dependencies
// esp-idf-sys = { version = "0.31.11", features = ["binstart"] }
// esp-idf-svc = "0.43.4"
// esp-idf-hal = "0.39.3"
// embedded-hal = "0.2.7"
// embedded-svc = "0.23.1"

// env
// . /home/lobanov/export-esp.sh
// fleashing
// espflash --monitor /dev/ttyUSB0 ./target/xtensa-esp32-espidf/debug/esp32-std-fft
// clear && cargo build && espflash --monitor /dev/ttyUSB0 ./target/xtensa-esp32-espidf/debug/esp32-std-fft

use std::{
    thread::sleep,
    time::Duration,
};
use esp_idf_sys as _;
use esp_idf_hal::{
    peripherals::Peripherals, 
    adc,
    gpio,
};
use esp_idf_sys::{self, *};

use esp_idf_svc::{
    wifi::EspWifi,
    nvs::EspDefaultNvsPartition,
    eventloop::EspSystemEventLoop,
};
use embedded_svc::wifi::{ClientConfiguration, Wifi, Configuration};

fn main(){
    esp_idf_sys::link_patches();//Needed for esp32-rs
    println!("Entered Main function!");
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    
    // #[cfg(esp32)]
    // let mut a2 = peripherals.pins.gpio34.into_analog_atten_11db()?;
    let mut adc1 = adc::AdcDriver::new(
        peripherals.adc1,
        &adc::config::Config::new().calibration(true),
    ).unwrap();


    let mut a2 = adc::AdcChannelDriver::<_, adc::Atten11dB<adc::ADC1>>::new(
        peripherals.pins.gpio34,
    ).unwrap();

    let mut wifi_driver = EspWifi::new(
        peripherals.modem,
        sys_loop,
        Some(nvs)
    ).unwrap();

    wifi_driver.set_configuration(&Configuration::Client(ClientConfiguration{
        ssid: "TKZ-2.4Gh".into(),
        password: "T2222222".into(),
        ..Default::default()
    })).unwrap();

    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();
    while !wifi_driver.is_connected().unwrap(){
        let config = wifi_driver.get_configuration().unwrap();
        println!("Waiting for station {:?}", config);
        sleep(Duration::new(1,0));
    }
    println!("Should be connected now");
    println!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info().unwrap());
    loop{
        sleep(Duration::from_secs_f64(0.5));
        match adc1.read(&mut a2) {
            Ok(value) => {
                println!("ADC1 read ch0 value: {:?}", value);

            },
            Err(err) => {
                println!("ADC1 read ch0 error: {:?}", err);
            },
        }
    }

}

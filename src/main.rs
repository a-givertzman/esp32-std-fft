#![allow(non_snake_case)]

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
    thread::{
        self,
        sleep,
    },
    time::Duration, net::{TcpListener, TcpStream, Ipv4Addr},
    io::{Result, Write, Read}, env,
};
use esp_idf_sys as _;
use esp_idf_hal::{
    peripherals::Peripherals, 
    adc,
    gpio::PinDriver, 
    // spi::Dma, 
    delay::FreeRtos,
};
// use esp_idf_sys::{self, *};

use esp_idf_svc::{
    wifi::EspWifi,
    nvs::EspDefaultNvsPartition,
    eventloop::EspSystemEventLoop,
};
use embedded_svc::wifi::{ClientConfiguration, Wifi, Configuration};

use heapless::{
    spsc::{
        Queue, 
    }, 
    Vec, 
};


const QSIZE: usize = 64;
const QSIZE_ADD: usize = QSIZE + 1;
static mut QUEUE: Queue<u16, QSIZE_ADD> = Queue::new();

fn main() {
    esp_idf_sys::link_patches();//Needed for esp32-rs
    println!("Entered Main function!");
    let peripherals = Peripherals::take().unwrap();
    
    let mut led = PinDriver::output(peripherals.pins.gpio4).unwrap();

    for _ in 0..4 {
        led.set_high().unwrap();
        // we are sleeping here to make sure the watchdog isn't triggered
        FreeRtos::delay_ms(100);

        led.set_low().unwrap();
        FreeRtos::delay_ms(100);
    }

    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    
    // #[cfg(esp32)]
    // let mut a2 = peripherals.pins.gpio34.into_analog_atten_11db()?;
    let adc_config = adc::config::Config::new()
        .calibration(true);
    let mut adc1 = adc::AdcDriver::new(
        peripherals.adc1,
        &adc_config,
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
    println!("Wifi should be connected now");
    let ipInfo = wifi_driver.sta_netif().get_ip_info().unwrap();

    println!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info().unwrap());
    test_tcp_bind(ipInfo.ip, 8080).unwrap();
    let queue = unsafe {&mut QUEUE};
    let cycleDelay = Duration::from_secs_f64(0.01);
    loop{
        sleep(cycleDelay);
        match adc1.read(&mut a2) {
            Ok(value) => {
                // println!("ADC1 read ch0 value: {:?}", value);
                match queue.enqueue(value) {
                    Ok(_) => {},
                    Err(err) => {
                        println!("add to queue error: {:?}", err);
                    },
                };            
            },
            Err(err) => {
                println!("ADC1 read ch0 error: {:?}", err);
            },
        }
    }

}


fn test_tcp_bind(ip: Ipv4Addr, port: u16) -> Result<()> {
    fn test_tcp_bind_accept(ip: Ipv4Addr, port: u16) -> Result<()> {
        println!("About to bind a simple echo service to port 8080");

        let listener = TcpListener::bind("0.0.0.0:8080")?;
        //     SocketAddr::V4(
        //         SocketAddrV4::new(
        //             ip, 
        //             port,
        //         )
        //     )
        // )?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("Accepted client");

                    thread::spawn(move || {
                        test_tcp_bind_handle_client(stream);
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }

        unreachable!()
    }

    fn test_tcp_bind_handle_client(mut stream: TcpStream) {
        // read 20 bytes at a time from stream echoing back to stream
        let queue = unsafe {&mut QUEUE};
        let cycleDelay = Duration::from_secs_f64(0.1);
        loop {
            let mut read = [0; 128];
            match stream.write_all(&read[0..1]) {
                Ok(_) => {}
                Err(err) => {
                    // connection was closed
                    println!("Socket stream write error (possible connection was closed): {}", err);
                    break;
                }
            };
            let qLen = queue.len();
            if qLen >= QSIZE {
                // let mut v: Vec<usize, QSIZE> = Vec::new();
                for _ in 0..qLen {
                    // v.push(queue.dequeue().unwrap()).unwrap();
                    let value = queue.dequeue().unwrap();
                    let bytes = value.to_be_bytes();
                    match stream.write_all(&bytes) {
                        Ok(_) => {}
                        Err(err) => {
                            // connection was closed
                            println!("Socket stream write error (possible connection was closed): {}", err);
                            break;
                        }        
                    }
                }
            };
            sleep(cycleDelay);
            // FreeRtos::delay_ms(100);

            // match stream.read(&mut read) {
            //     Ok(n) => {
            //         if n == 0 {
            //             // connection was closed
            //             break;
            //         }
            //         stream.write_all(&read[0..n]).unwrap();
            //     }
            //     Err(err) => {
            //         panic!("{}", err);
            //     }
            // }
        }
    }

    thread::spawn(move || test_tcp_bind_accept(ip, port).unwrap());

    Ok(())
}
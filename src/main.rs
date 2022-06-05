use clap::{self, Arg, Command};
use env_logger::Builder;
use log::LevelFilter;
use serial::prelude::*;
use std::io::Read;
use std::io::Write;
use std::time::Duration;

const RETRIES: u8 = 5;

fn get_args() -> clap::ArgMatches {
    Command::new("ToDo-Threader")
        .version(clap::crate_version!())
        .author(clap::crate_authors!("\n"))
        .about(clap::crate_description!())
        .arg(
            Arg::new("Serial_Port")
                .help("The Serial Port address for the Device")
                .required(true)
                .takes_value(true)
                .value_name("ADDRESS"),
        )
        .arg(
            Arg::new("Baud_Rate")
                .short('B')
                .long("baud-rate")
                .help("Set the Baud Rate for communications")
                .takes_value(true)
                .value_name("BAUD")
                .default_value("9600"),
        )
        .arg(
            Arg::new("Timeout")
                .short('T')
                .long("timeout")
                .help("The Timout in milliseconds for communications")
                .takes_value(true)
                .value_name("TIMEOUT")
                .default_value("500"),
        )
        .arg(
            Arg::new("Debug")
                .short('d')
                .long("debug")
                .help("Set Log Level to Debug"),
        )
        .arg(
            Arg::new("Test")
                .short('t')
                .long("test")
                .help("Send a test to see if the device is available")
                .conflicts_with_all(&["Next", "Following", "Add"]),
        )
        .arg(
            Arg::new("Raw")
                .short('r')
                .long("raw")
                .help("Send a payload directly")
                .takes_value(true)
                .value_name("PAYLOAD")
                .conflicts_with_all(&["Test", "Next", "Following", "Add"]),
        )
        .arg(
            Arg::new("Next")
                .short('n')
                .long("next")
                .help("Send a next command to mark the current task as done")
                .conflicts_with_all(&["Test", "Following", "Add"]),
        )
        .arg(
            Arg::new("Following")
                .short('f')
                .long("following")
                .help("Set a task and schedule it as the next one")
                .takes_value(true)
                .value_name("TASK")
                .requires("Color")
                .conflicts_with_all(&["Test", "Next", "Add"]),
        )
        .arg(
            Arg::new("Add")
                .short('a')
                .long("add")
                .help("Set a task and schedule it at the end")
                .takes_value(true)
                .value_name("TASK")
                .requires("Color")
                .conflicts_with_all(&["Test", "Next", "Following"]),
        )
        .arg(
            Arg::new("Color")
                .short('c')
                .long("color")
                .help("Set the color for a new Task in HTML notation")
                .takes_value(true)
                .value_name("COLOR")
                .default_value("#FFFFFF"),
        )
        .get_matches()
}

fn init_logger(level: LevelFilter) {
    Builder::new()
        .format(|buf, record| writeln!(buf, "[{}]: {}", record.level(), record.args(),))
        .filter(None, level)
        .init();
}

fn init_comm_settings(baudrate: usize) -> serial::PortSettings {
    let baud = match baudrate {
        110 => serial::Baud110,
        300 => serial::Baud300,
        1200 => serial::Baud1200,
        2400 => serial::Baud2400,
        4800 => serial::Baud4800,
        9600 => serial::Baud9600,
        19200 => serial::Baud19200,
        38400 => serial::Baud38400,
        57600 => serial::Baud57600,
        115200 => serial::Baud115200,
        b => serial::BaudOther(b),
    };

    serial::PortSettings {
        baud_rate: baud,
        char_size: serial::Bits8,
        parity: serial::ParityNone,
        stop_bits: serial::Stop1,
        flow_control: serial::FlowNone,
    }
}

fn init_communication(
    address: &str,
    settings: &serial::PortSettings,
    timeout: u64,
) -> Result<serial::SystemPort, serial::Error> {
    let mut device = serial::open(address)?;
    device.configure(settings)?;
    device.set_timeout(Duration::from_millis(timeout))?;

    Ok(device)
}

fn main() {
    // Load command line arguments
    let args = get_args();

    // Build the logger
    let llvl;
    if args.is_present("Debug") {
        llvl = LevelFilter::Debug;
    } else {
        llvl = LevelFilter::Info;
    }
    init_logger(llvl);

    // Build Communication Settings
    let settings = init_comm_settings(
        args.value_of("Baud_Rate")
            .expect("Unexpected")
            .parse::<usize>()
            .expect("Provided Baud Rate must be an integer"),
    );
    let timeout = args
        .value_of("Timeout")
        .expect("Unexpected")
        .parse::<u64>()
        .expect("Provided Timeout must be an integer");
    let address = args.value_of("Serial_Port").expect("Unexpected");

    let mut device = match init_communication(&address, &settings, timeout) {
        Ok(val) => val,
        Err(err) => panic!(
            "Failed to initialize communication with the Device! Reason: {}",
            err
        ),
    };

    if args.is_present("Test") {
        test(&mut device)
    } else if args.is_present("Raw") {
        raw(&mut device, args.value_of("Raw").expect("Unexpected"));
    } else if args.is_present("Next") || args.is_present("Following") || args.is_present("Add") {
        for i in 0..RETRIES {
            let val = if args.is_present("Next") {
                next(&mut device)
            } else if args.is_present("Following") {
                following(
                    &mut device,
                    args.value_of("Following").expect("Unexpected"),
                    args.value_of("Color").expect("Unexpected"),
                )
            } else {
                add(
                    &mut device,
                    args.value_of("Add").expect("Unexpected"),
                    args.value_of("Color").expect("Unexpected"),
                )
            };
            if val.is_ok() {
                log::info!("Success!");
                break;
            } else {
                log::warn!("Failed to communicate! Reason: {:?}", val);
                log::info!("Retrying... {}/{}", i + 1, RETRIES);
            }
        }
    }
}

fn test(device: &mut serial::SystemPort) {
    log::debug!("Starting communication test...");
    log::debug!("Sending data to device...");
    match device.write("ping".as_bytes()) {
        Ok(num) => {
            log::debug!("Successfully sent {} bytes to the device", num);
            log::info!("Writing . . . . . [ OK ]");
        }
        Err(err) => {
            log::info!("Failed to sent data to the device! Reason: {}", err);
            log::error!("Writing . . . . . [ FAILED ]");
        }
    }
    log::debug!("Reading data from device...");
    let mut read_buffer = [0u8; 1];
    match device.read(&mut read_buffer) {
        Ok(num) => {
            log::debug!("Successfully read {} bytes from the device", num);
            log::info!("Reading . . . . . [ OK ]");
        }
        Err(err) => {
            log::info!("Failed to read data from the device! Reason: {}", err);
            log::error!("Reading . . . . . [ FAILED ]")
        }
    }
    log::debug!("Communication test finished");
}

fn raw(device: &mut serial::SystemPort, payload: &str) {
    match device.write(payload.as_bytes()) {
        Ok(num) => log::info!("Successfully sent {} bytes to the device", num),
        Err(err) => log::error!("Failed to sent data to the device! Reason: {}", err),
    }
    let mut read_buffer = [0u8; 1];
    match device.read(&mut read_buffer) {
        Ok(num) => log::info!(
            "Got a response of {} bytes from device:\n{:?}",
            num,
            read_buffer
        ),
        Err(err) => log::error!("Failed to read data from device! Reason: {}", err),
    }
}

fn next(device: &mut serial::SystemPort) -> Result<(), std::io::Error> {
    device.write("NXT".as_bytes())?;
    let mut read_buffer = [0u8; 1];
    device.read(&mut read_buffer)?;
    Ok(())
}

fn following(
    device: &mut serial::SystemPort,
    message: &str,
    color: &str,
) -> Result<(), std::io::Error> {
    device.write(
        format!(
            "FLW{};{}",
            message,
            color.strip_prefix("#").unwrap_or(color)
        )
        .as_bytes(),
    )?;
    let mut read_buffer = [0u8; 1];
    device.read(&mut read_buffer)?;
    Ok(())
}

fn add(device: &mut serial::SystemPort, message: &str, color: &str) -> Result<(), std::io::Error> {
    device.write(
        format!(
            "ADD{};{}",
            message,
            color.strip_prefix("#").unwrap_or(color)
        )
        .as_bytes(),
    )?;
    let mut read_buffer = [0u8; 1];
    device.read(&mut read_buffer)?;
    Ok(())
}

use std::error::Error;
use std::ffi::c_int;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

const VENDOR_ID: &str = "00001A2C";
const PRODUCT_ID: &str = "0000434D";
const REPORT_ID: u8 = 0x07;
const REPORT_LEN: usize = 64;
const HIDIOCSFEATURE_64: usize = 0xC040_4806;

#[derive(Clone, Copy)]
enum Unit {
    Celsius,
    Fahrenheit,
}

extern "C" {
    fn ioctl(fd: c_int, request: usize, ...) -> c_int;
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ocypus-a40: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let Some(argument) = args.next() else {
        return Err("usage: ocypus-a40 <temperature 0..199|cpu|gpu> [--fahrenheit]".into());
    };
    let unit = match args.next().as_deref() {
        None => Unit::Celsius,
        Some("--fahrenheit") => Unit::Fahrenheit,
        Some(_) => return Err("the only supported option is --fahrenheit".into()),
    };

    let device = find_feature_device()?;
    let file = OpenOptions::new().read(true).write(true).open(&device)?;
    if argument == "cpu" {
        let sensor = find_cpu_sensor()?;
        return run_stream(&file, &device, "CPU", unit, || read_cpu_temperature(&sensor));
    }
    if argument == "gpu" {
        return run_stream(&file, &device, "GPU", unit, read_gpu_temperature);
    }

    let temperature = display_temperature(parse_temperature(&argument)?, unit)?;
    send_temperature(&file, temperature)?;
    println!("Sent {temperature} {} using {}", unit.name(), device.display());
    Ok(())
}

fn run_stream<F>(
    file: &fs::File,
    device: &Path,
    label: &str,
    unit: Unit,
    mut read_temperature: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut() -> Result<u8, Box<dyn Error>>,
{
    loop {
        let temperature = display_temperature(read_temperature()?, unit)?;
        send_temperature(file, temperature)?;
        println!("{label} {temperature} {} -> {}", unit.name(), device.display());
        thread::sleep(Duration::from_secs(1));
    }
}

impl Unit {
    fn name(self) -> &'static str {
        match self {
            Unit::Celsius => "C",
            Unit::Fahrenheit => "F",
        }
    }
}

fn display_temperature(celsius: u8, unit: Unit) -> Result<u8, Box<dyn Error>> {
    let value = match unit {
        Unit::Celsius => celsius as u16,
        Unit::Fahrenheit => (celsius as u16 * 9 / 5) + 32,
    };
    if value > 199 {
        return Err("converted temperature exceeds the display's 0..199 range".into());
    }
    Ok(value as u8)
}

fn send_temperature(file: &fs::File, temperature: u8) -> io::Result<()> {
    if temperature > 199 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "temperature must be between 0 and 199",
        ));
    }
    let mut report = make_report(temperature);
    let result = unsafe {
        ioctl(
            file.as_raw_fd(),
            HIDIOCSFEATURE_64,
            report.as_mut_ptr(),
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn find_cpu_sensor() -> Result<PathBuf, Box<dyn Error>> {
    for entry in fs::read_dir("/sys/class/hwmon")? {
        let hwmon = entry?.path();
        if fs::read_to_string(hwmon.join("name"))?.trim() != "coretemp" {
            continue;
        }
        for label in fs::read_dir(&hwmon)? {
            let label = label?.path();
            let Some(name) = label.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.ends_with("_label") && fs::read_to_string(&label)?.trim() == "Package id 0" {
                let input_name = name.trim_end_matches("_label").to_owned() + "_input";
                let input = hwmon.join(input_name);
                if input.is_file() {
                    return Ok(input);
                }
            }
        }
    }
    Err("no Intel coretemp Package id 0 sensor found under /sys/class/hwmon".into())
}

fn read_cpu_temperature(sensor: &Path) -> Result<u8, Box<dyn Error>> {
    let milli_celsius: u32 = fs::read_to_string(sensor)?.trim().parse()?;
    let celsius = (milli_celsius / 1000).min(199) as u8;
    Ok(celsius)
}

fn read_gpu_temperature() -> Result<u8, Box<dyn Error>> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()?;
    if !output.status.success() {
        return Err("nvidia-smi failed to read the GPU temperature".into());
    }
    parse_temperature(std::str::from_utf8(&output.stdout)?)
}

fn parse_temperature(value: &str) -> Result<u8, Box<dyn Error>> {
    let temperature: u8 = value.trim().parse()?;
    if temperature > 199 {
        return Err("temperature must be between 0 and 199".into());
    }
    Ok(temperature)
}

fn find_feature_device() -> Result<PathBuf, Box<dyn Error>> {
    let mut matches = Vec::new();
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let entry = entry?;
        let name = entry.file_name();
        let metadata = fs::read_to_string(entry.path().join("device/uevent"))?;
        let expected_id = format!("HID_ID=0003:{VENDOR_ID}:{PRODUCT_ID}");
        if !metadata.lines().any(|line| line == expected_id) {
            continue;
        }

        let descriptor = fs::read(entry.path().join("device/report_descriptor"))?;
        if has_feature_report(&descriptor, REPORT_ID, 63) {
            matches.push(PathBuf::from("/dev").join(name));
        }
    }

    match matches.as_slice() {
        [device] => Ok(device.clone()),
        [] => Err("no matching 1A2C:434D HID feature-report interface found".into()),
        _ => Err("multiple matching HID feature-report interfaces found".into()),
    }
}

fn has_feature_report(descriptor: &[u8], report_id: u8, expected_bytes: u32) -> bool {
    let (mut current_id, mut report_size, mut report_count) = (0u8, 0u32, 0u32);
    let mut offset = 0;

    while offset < descriptor.len() {
        let prefix = descriptor[offset];
        offset += 1;
        if prefix == 0xFE {
            if offset + 2 > descriptor.len() {
                return false;
            }
            let length = descriptor[offset] as usize;
            offset += 2 + length;
            if offset > descriptor.len() {
                return false;
            }
            continue;
        }

        let size = match prefix & 0x03 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        if offset + size > descriptor.len() {
            return false;
        }
        let mut value = 0u32;
        for (index, byte) in descriptor[offset..offset + size].iter().enumerate() {
            value |= (*byte as u32) << (index * 8);
        }
        offset += size;

        match ((prefix >> 2) & 0x03, prefix >> 4) {
            (1, 7) => report_size = value,
            (1, 8) => current_id = value as u8,
            (1, 9) => report_count = value,
            (0, 11) if current_id == report_id => {
                return report_size.saturating_mul(report_count) == expected_bytes * 8;
            }
            _ => {}
        }
    }
    false
}

fn make_report(temperature: u8) -> [u8; REPORT_LEN] {
    let mut report = [0; REPORT_LEN];
    report[0] = REPORT_ID;
    report[1] = 0xff;
    report[2] = 0xff;
    report[3] = temperature / 100;
    report[4] = (temperature / 10) % 10;
    report[5] = temperature % 10;
    report
}

#[cfg(test)]
mod tests {
    use super::{display_temperature, has_feature_report, make_report, parse_temperature, Unit};

    #[test]
    fn report_encodes_numeric_digits_and_zero_padding() {
        let report = make_report(45);
        assert_eq!(&report[..6], &[0x07, 0xff, 0xff, 0, 4, 5]);
        assert!(report[6..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn report_encodes_three_digit_values() {
        assert_eq!(&make_report(199)[3..6], &[1, 9, 9]);
    }

    #[test]
    fn parses_temperature_from_sensor_output() {
        assert_eq!(parse_temperature("48\n").unwrap(), 48);
        assert!(parse_temperature("N/A\n").is_err());
    }

    #[test]
    fn converts_celsius_to_fahrenheit_like_windows_baseline() {
        assert_eq!(display_temperature(44, Unit::Fahrenheit).unwrap(), 111);
        assert_eq!(display_temperature(37, Unit::Fahrenheit).unwrap(), 98);
        assert!(display_temperature(94, Unit::Fahrenheit).is_err());
    }

    #[test]
    fn descriptor_matches_only_feature_report_id_seven() {
        let descriptor = [
            0x06, 0x01, 0xff, 0x09, 0x01, 0xa1, 0x01, 0x85, 0x07, 0x09, 0x03, 0x15, 0x00,
            0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x3f, 0xb1, 0x02, 0xc0,
        ];
        assert!(has_feature_report(&descriptor, 7, 63));
        assert!(!has_feature_report(&descriptor, 7, 64));
        assert!(!has_feature_report(&descriptor, 6, 63));
    }
}
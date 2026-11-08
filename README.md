# Ocypus Iota A40 for Linux

A small Rust utility for sending temperatures to the Ocypus Iota A40 cooler's
dot-matrix display over its HID feature-report interface. It uses Linux
`hidraw` and has no third-party Rust dependencies.

## Requirements

- Linux with Rust and Cargo installed.
- An Ocypus Iota A40 connected over USB (USB ID `1a2c:434d`).
- For CPU mode: an Intel CPU exposing the `coretemp` `Package id 0` sensor.
- For GPU mode: NVIDIA drivers and the `nvidia-smi` command.

## Install

From the repository root, run:

```sh
sh ./install.sh
```

The script builds the release binary, installs it to `/usr/local/bin`, and
installs the udev rule to grant the active desktop user access to matching
hidraw devices. It uses `sudo` for these system-wide changes.

**Permission scope:** the rule matches USB product `1a2c:434d` and therefore
grants access to both HID interfaces of the composite device, not only the
display interface. Review [`udev/99-ocypus-a40.rules`](udev/99-ocypus-a40.rules)
before installing if this scope is not appropriate for your system.

## Usage

Run the installed command as your regular user:

```sh
ocypus-a40 45
ocypus-a40 cpu
ocypus-a40 gpu
ocypus-a40 cpu --fahrenheit
```

- A number sends one temperature from 0 to 199.
- `cpu` reads Intel `coretemp` `Package id 0` once per second.
- `gpu` reads NVIDIA GPU temperature using `nvidia-smi` once per second.
- `--fahrenheit` converts the sensor value before sending it.
- Press Ctrl+C to stop a live stream.

## Development

Build and run the checks with the Rust toolchain:

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

The hardware is not required to compile or run the test suite. Physical-device
checks require the cooler and the relevant temperature sensor.
See [`CONTRIBUTING.md`](CONTRIBUTING.md) for contribution guidelines.

## Project layout

```text
.
├── .github/workflows/ci.yml  # Linux formatting, lint, and test checks
├── analysis/                 # Local-only reverse-engineering inputs and tools
├── docs/                     # Implementation evidence and research notes
├── src/                      # Rust application
├── udev/                     # Device-permission rule
├── CONTRIBUTING.md
├── install.sh
└── README.md
```

Files under `analysis/` are intentionally excluded from version control.
See [`analysis/README.md`](analysis/README.md) for what belongs there and how
to work with the local materials. The project documentation does not depend on
having those files.

## Current limitations

- CPU mode currently supports the Intel `coretemp` `Package id 0` sensor only.
- AMD CPU sensors and startup integration are not implemented.
- GPU mode supports NVIDIA through `nvidia-smi`.
- The report format and remaining unknowns are documented in
  [`docs/reverse-engineering.md`](docs/reverse-engineering.md).

## Uninstall

Remove the installed binary and udev rule, then reload the rules:

```sh
sudo rm /usr/local/bin/ocypus-a40 /etc/udev/rules.d/99-ocypus-a40.rules
sudo udevadm control --reload-rules
```

Log out and back in, or reconnect the cooler, to clear the active-session
device access grant.

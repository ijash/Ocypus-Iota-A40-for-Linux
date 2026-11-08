# Reverse-Engineering Notes

This document records observed behavior, implementation evidence, and
unresolved research questions. Claims described as pending or unverified have
not been confirmed on the physical display.

## Target and Method

The target is Ocypus Digital setup 1.0.0.19 for the Iota A40 cooler. The
installer and Windows application were not executed. The installer was
extracted for static analysis; Windows display behavior was recorded
separately. Linux tests use the connected USB device and the extracted
application as separate evidence sources.

## Confirmed Windows Behavior

- The Windows baseline showed CPU values around 44-45, GPU 37, and
  Fahrenheit 111 before returning to Celsius 44.
- The application enumerates HID devices and checks USB ID `1a2c:434d`, despite
  the operating system describing it as a USB gaming keyboard.
- In the temperature update routine at `0x407050`, the application constructs
  a report beginning `07 FF FF`, followed by three numeric digit bytes, and
  updates once per second.
- The write path at `0x4091e0` passes a length of `0x41` (65) to the native HID
  feature-report wrapper.
- A separate `07 FD` request is used in a version-check path; it is not
  established as a display update command.

## Linux HID Evidence

- Linux enumerates `1a2c:434d` as a two-interface HID composite device. The
  keyboard interface does not advertise the feature report used here.
- Interface 1 is exposed as `/dev/hidraw1` on the current machine. Its report
  descriptor declares feature report ID `0x07` with 63 data bytes, for 64 bytes
  total including the report ID.
- Linux's hidraw node was root-only at the time of testing. The implementation
  discovers the interface by USB identity and descriptor rather than relying
  on the unstable node number.
- The Linux program sent a 64-byte report beginning `07 FF FF 00 04 05`, and
  the cooler displayed 45 degrees. This verifies the descriptor-sized Linux
  report and the numeric digit encoding.
- CPU mode successfully read Intel `coretemp` `Package id 0` values and sent
  them once per second. The terminal showed changing values (43-52 C); visual
  confirmation that the display followed this stream is still pending.
- GPU mode reads `nvidia-smi` temperature output. On this host the read-only
  query returned 48 C; visual confirmation of GPU mode on the display is still
  pending.

## Implementation

`src/main.rs` uses the Linux `HIDIOCSFEATURE(64)` ioctl directly, so there are
no third-party Rust dependencies. It identifies report ID 7 from the HID report
descriptor, creates a zero-padded 64-byte report, and writes either one
caller-supplied temperature or a live Intel CPU or NVIDIA GPU temperature.

The app's 65-byte Windows call and the descriptor's 64-byte Linux report differ
by one byte. Linux accepted the descriptor-sized report and displayed the
expected value; the extra Windows byte's purpose remains unknown.

## Unverified or Excluded Claims

The prior experiment's 72-byte USB control transfers,
repeating response pattern, and `Celsius + 2265` encoding are not used. Their
relationship to the successful Windows HID path has not been established.
Likewise, `26ce:01a2` is identified by Linux as an ASRock LED Controller and is
not treated as the cooler.

## Next Research Steps

1. Confirm visually that live CPU mode tracks changes on the cooler.
2. Compare Linux output with Windows for Fahrenheit behavior and confirm GPU
   mode visually.
3. Add AMD CPU sensor support and broader Ubuntu testing.
4. Review the optional udev permission scope before installing it.

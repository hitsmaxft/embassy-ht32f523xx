# HT32 Raw HID full-load validation

This firmware exposes one vendor-defined Raw HID interface. It intentionally
contains no keyboard, mouse, or consumer-control usages, so running the test
cannot inject system input.

Device identity:

- Product: `BHE HT32F52352 RAW HID SPEED`
- Serial: `BHE-HT32F52352-RAWHID-0001`
- Vendor usage page: `0xFF00`
- Reports: 64-byte interrupt OUT and IN, 1 ms polling interval

Build and flash with the Holtek CMSIS-DAP probe:

```sh
cargo build --release -p usb-raw-hid-speed-test
probe-rs run --chip HT32F52352 \
  --probe 04d9:802f:02000D2E --protocol swd --speed 1000 --verify \
  target/thumbv6m-none-eabi/release/usb-raw-hid-speed-test
```

Run the host integrity and throughput test in an isolated Python environment:

```sh
uvx --from hidapi python host_test.py --packets 20000
```

At Full-Speed, a 64-byte interrupt endpoint polled every 1 ms has a nominal
application-payload ceiling of 64,000 bytes/s in each direction. The host test
prints the measured per-direction and aggregate rates and rejects reordered or
corrupted packets.

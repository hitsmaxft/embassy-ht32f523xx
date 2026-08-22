#!/usr/bin/env python3
"""Correctness and full-load Raw HID echo validation for the HT32 test firmware."""

import argparse
import os
import struct
import threading
import time

import hid

VID = 0x16C0
PID = 0x05DC
SERIAL = "BHE-HT32F52352-RAWHID-0001"
REPORT_SIZE = 64


def make_packet(sequence: int) -> bytes:
    header = struct.pack("<I", sequence)
    payload = bytes(((sequence * 17 + i * 29) & 0xFF) for i in range(REPORT_SIZE - 4))
    return header + payload


def find_device() -> dict:
    matches = [
        item
        for item in hid.enumerate(VID, PID)
        if item.get("serial_number") == SERIAL
    ]
    if len(matches) != 1:
        raise RuntimeError(f"expected exactly one {SERIAL!r} device, found {len(matches)}")
    return matches[0]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--packets", type=int, default=20_000)
    parser.add_argument("--timeout", type=float, default=30.0)
    args = parser.parse_args()

    info = find_device()
    print(
        "DEVICE",
        f"manufacturer={info.get('manufacturer_string')!r}",
        f"product={info.get('product_string')!r}",
        f"serial={info.get('serial_number')!r}",
        f"usage_page=0x{info.get('usage_page', 0):04x}",
    )

    device = hid.device()
    device.open_path(info["path"])
    device.set_nonblocking(False)

    # First prove exact end-to-end payload integrity without pipelining.
    for sequence in range(16):
        packet = make_packet(sequence)
        written = device.write(b"\x00" + packet)
        if written not in (REPORT_SIZE, REPORT_SIZE + 1):
            raise RuntimeError(f"short HID write: {written}")
        received = bytes(device.read(REPORT_SIZE, int(args.timeout * 1000)))
        if received != packet:
            raise RuntimeError(
                f"echo mismatch at sequence {sequence}: got={received.hex()} expected={packet.hex()}"
            )
    print("INTEGRITY PASS packets=16 bytes=1024")

    sent = 0
    received = 0
    failure = []
    started = time.monotonic()

    def writer() -> None:
        nonlocal sent
        try:
            for sequence in range(16, 16 + args.packets):
                written = device.write(b"\x00" + make_packet(sequence))
                if written not in (REPORT_SIZE, REPORT_SIZE + 1):
                    raise RuntimeError(f"short HID write: {written}")
                sent += 1
        except BaseException as exc:
            failure.append(exc)

    def reader() -> None:
        nonlocal received
        try:
            for sequence in range(16, 16 + args.packets):
                packet = bytes(device.read(REPORT_SIZE, int(args.timeout * 1000)))
                expected = make_packet(sequence)
                if packet != expected:
                    raise RuntimeError(
                        f"stream mismatch at sequence {sequence}: got={packet.hex()} expected={expected.hex()}"
                    )
                received += 1
        except BaseException as exc:
            failure.append(exc)

    writer_thread = threading.Thread(target=writer, name="raw-hid-writer")
    reader_thread = threading.Thread(target=reader, name="raw-hid-reader")
    writer_thread.start()
    reader_thread.start()
    writer_thread.join(args.timeout)
    reader_thread.join(args.timeout)
    elapsed = time.monotonic() - started
    device.close()

    if writer_thread.is_alive() or reader_thread.is_alive():
        raise RuntimeError(f"throughput test timed out: sent={sent} received={received}")
    if failure:
        raise failure[0]
    if sent != args.packets or received != args.packets:
        raise RuntimeError(f"packet count mismatch: sent={sent} received={received}")

    byte_count = args.packets * REPORT_SIZE
    print(
        "THROUGHPUT PASS",
        f"packets={args.packets}",
        f"elapsed_s={elapsed:.3f}",
        f"out_pps={sent / elapsed:.1f}",
        f"in_pps={received / elapsed:.1f}",
        f"out_Bps={byte_count / elapsed:.1f}",
        f"in_Bps={byte_count / elapsed:.1f}",
        f"aggregate_Bps={2 * byte_count / elapsed:.1f}",
    )


if __name__ == "__main__":
    main()

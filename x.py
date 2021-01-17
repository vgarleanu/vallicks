#!/usr/bin/python

from subprocess import check_call
from pathlib import Path
from argparse import BooleanOptionalAction

import os
import sys
import argparse

assert sys.version_info >= (3, 9)


parser = argparse.ArgumentParser()
parser.add_argument("--qemu", action=BooleanOptionalAction)

args = parser.parse_args()


build_path = Path(".").joinpath("build")

common_path = Path(__file__).parent

arch = "x86_64"
mode = "debug"

kernel_blob = build_path.joinpath(f"kernel-{arch}-{mode}.bin")
grub_cfg = common_path.joinpath("grub.cfg")
linker_script = common_path.joinpath("x86_64.linker.ld")

libkernel_path = Path(f"target/target-x86_64/{mode}/vallicks").absolute()
iso_path = Path(f"target/target-x86_64/{mode}/bootimage-vallicks.bin").absolute()


def sh(st: str):
    return check_call(st, shell=True)


if args.qemu:
    sh("cargo bootimage")

    assert libkernel_path.exists()

    qemu_args = os.environ.get("QEMU_ARGS", None)

    if qemu_args is None:
        memory = os.environ.get("QEMU_MEM", "512M")
        smp = os.environ.get("QEMU_SMP", "4")

        qemu_args = " ".join([
            f"-m {memory}",
            f"-smp {smp}",
            "-s",
            "-vga std",
            "-nographic",
            "-device rtl8139,netdev=u1,mac=12:34:56:67:90:ab",
            "-netdev tap,id=u1,ifname=tap0,script=no,downscript=no",
            "-object filter-dump,id=f1,netdev=u1,file=dump.dat",
            "-no-reboot",
            "-no-shutdown",
        ])

    sh(f"qemu-system-x86_64 -drive format=raw,file={iso_path} {qemu_args}")

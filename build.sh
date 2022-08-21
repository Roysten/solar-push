#!/bin/sh

CROSS_COMPILE=/mnt/data/arm-linux-musleabihf-cross/bin/arm-linux-musleabihf- cargo build --target=arm-unknown-linux-musleabihf --release


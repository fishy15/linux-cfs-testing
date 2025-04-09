#!/bin/bash

set -x

cd $(dirname $0)/../vm

if [ ! -z $K ]; then
    kern="-kernel ../kbuild/arch/x86_64/boot/bzImage -append"
    args="root=/dev/sda1 console=ttyS0,115200 nokaslr"
else
    sudo=sudo
    kvm='-accel kvm'
fi
    
$sudo qemu-system-x86_64 $kern "$args" $kvm -nographic -hda d.q -device virtio-net-pci,netdev=net0 -netdev user,id=net0,hostfwd=tcp::2222-:22 -smp 16 -m 16G $@

#!/bin/bash

set -x

cd $(dirname $0)/../vm

if [ -z "$SSH" ]; then
	SSH=2222
fi

if [ -z "$TOPOLOGY" ]; then
	echo "using a default number of 2 cores"
	topology="-smp cpus=2"
elif [ "$TOPOLOGY" = "16-tiered" ]; then
	topology="\
		-object memory-backend-ram,size=4G,id=m0 \
		-object memory-backend-ram,size=4G,id=m1 \
		-object memory-backend-ram,size=4G,id=m2 \
		-object memory-backend-ram,size=4G,id=m3 \
		-numa node,cpus=0-3,nodeid=0,memdev=m0 \
		-numa node,cpus=4-7,nodeid=1,memdev=m1 \
		-numa node,cpus=8-11,nodeid=2,memdev=m2 \
		-numa node,cpus=12-15,nodeid=3,memdev=m3 \
		-smp cpus=16"
else
	topology="-smp cpus=$TOPOLOGY"
fi

if [ ! -z $K ]; then
	kern="-kernel ../kbuild/arch/x86_64/boot/bzImage -append"
	args="'root=/dev/sda1 console=ttyS0,115200 nokaslr'"
else
	sudo=sudo
	kvm='-accel kvm'
fi
	
sh -c "$sudo qemu-system-x86_64 $kern $args $kvm -nographic -device virtio-net-pci,netdev=net0 -netdev user,id=net0,hostfwd=tcp::$SSH-:22 $topology -m 16G d.q $@"

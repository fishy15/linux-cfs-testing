#!/bin/bash

set -x
set -e

cd $(dirname $0)/..
[ ! -d deb_build ]
mkdir -p deb_build/kbuild
cd deb_build/kbuild
make -C ../../linux O=$(pwd) LLVM=1 defconfig
cp ../../linux/myconfig .config
make LLVM=1 -j`nproc` deb-pkg
cd ..
ssh -p2222 root@localhost '[ ! -d deb_build ]'
tar cf - linux-* | ssh -p2222 root@localhost 'mkdir deb_build && cd deb_build && tar xf - && dpkg -i *.deb && cd .. && rm -rf deb_build && reboot'
cd ..
rm -rf deb_build

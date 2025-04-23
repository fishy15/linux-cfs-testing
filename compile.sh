#!/bin/bash

set -x

cd $(dirname $0)/../kbuild

make -C ../linux O=$(pwd) defconfig
cp ../linux/myconfig .config
LLVM=1 make -j16

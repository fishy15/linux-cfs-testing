#!/bin/bash

set -x
set -e

export PATH=$HOME/.cargo/bin:$PATH

# this should be ~/linux
cd $(dirname $0)

rm -rf ../kbuild && mkdir ../kbuild

# install rustup
which rustup || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# install clang
sudo apt install -y clang lld debhelper-compat
# install debhelper-compat for make deb-pkg too^

# get matching toolchain for clang 14
rustup toolchain install $(scripts/min-tool-version.sh rustc)
rustup default $(scripts/min-tool-version.sh rustc)

# get bindgen, etc
cargo install --locked bindgen-cli@$(scripts/min-tool-version.sh bindgen)
rustup component add clippy rustfmt rust-src

# make kbuild directory
cd ../kbuild
make LLVM=1 -C ../linux O=$(pwd) defconfig
cp ../linux/myconfig .config
make LLVM=1 rustavailable

echo "export PATH=$HOME/.cargo/bin:$PATH" >> ~/.bashrc

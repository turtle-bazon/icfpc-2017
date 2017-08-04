#!/bin/bash

DSTDIR="$1"

mkdir -p $DSTDIR

cd lambda_punter_offline/
cargo build --release || exit $?
cargo test --release || exit $?
install -m 755 target/release/lambda_punter_offline $DSTDIR/punter
cargo clean
cd ..

cd lambda_punter/
cargo clean
cd ..

touch $DSTDIR/install
chmod +x $DSTDIR/install
install -m 644 PACKAGES $DSTDIR
install -m 644 README.md $DSTDIR/README
mkdir -p $DSTDIR/src
cp -r lambda_punter $DSTDIR/src
cp -r lambda_punter_offline $DSTDIR/src

exit 0

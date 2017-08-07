#!/bin/bash

cd lambda_punter_offline/
cargo update
cargo build --release
cd ..

tools/benchmark/benchmark.php $*

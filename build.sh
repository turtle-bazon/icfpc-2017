#!/bin/bash

cd lambda_punter/
cargo build || exit $?
cargo test || exit $?
cd ..

cd lambda_punter_offline/
cargo build || exit $?
cargo test || exit $?
cd ..

exit 0

#!/bin/bash

cd lambda_punter/

cargo build || exit $?
cargo test || exit $?

exit 0

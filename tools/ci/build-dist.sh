#!/bin/bash

TEMP_DIR=`mktemp -d`

cat << '!!!' | base64 -d > $TEMP_DIR/build.tar

#!/bin/bash

REPO_URL="$1"
BUILD_REF="$2"

TEMP_DIR=`mktemp -d`
TEMP_GIT=$TEMP_DIR/git

git clone --mirror $REPO_URL $TEMP_GIT
git --git-dir=$TEMP_GIT archive --format=tar $BUILD_REF > $TEMP_DIR/build.tar

cat build-dist.sh > $TEMP_DIR/build.sh
base64 -w 0 $TEMP_DIR/build.tar >> $TEMP_DIR/build.sh
echo >> $TEMP_DIR/build.sh
echo '!!!' >> $TEMP_DIR/build.sh
cat build-dist-tail.sh >> $TEMP_DIR/build.sh

cat $TEMP_DIR/build.sh | ssh punter@localhost -p 2222
RETVAL=$?

rm -rf $TEMP_DIR

exit $RETVAL

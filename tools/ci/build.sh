#!/bin/bash

REPO_URL="$1"
BUILD_REF="$2"

TEMP_DIR=`mktemp -d`
TEMP_GIT=$TEMP_DIR/git

git clone --mirror $REPO_URL $TEMP_GIT
git --git-dir=$TEMP_GIT archive --format=tar $BUILD_REF | gzip > $TEMP_DIR/build.tar.gz

cat > $TEMP_DIR/build.sh <<\EOF
#!/bin/bash

TEMP_DIR=`mktemp -d`

cat << '!!!' | base64 -d > $TEMP_DIR/build.tar.gz
EOF
base64 -w 0 $TEMP_DIR/build.tar.gz >> $TEMP_DIR/build.sh
echo >> $TEMP_DIR/build.sh
echo '!!!' >> $TEMP_DIR/build.sh

cat >> $TEMP_DIR/build.sh <<\EOF
cd $TEMP_DIR
tar xf build.tar.gz
rm -f build.tar.gz

./build.sh 2>&1
RETVAL=$?

cd /
rm -rf $TEMP_DIR

exit $RETVAL
EOF

cat $TEMP_DIR/build.sh | ssh punter@localhost -p 2222
RETVAL=$?

rm -rf $TEMP_DIR

exit $RETVAL


cd $TEMP_DIR
tar xf build.tar
rm -f build.tar

./build.sh
RETVAL=$?

cd /
rm -rf $TEMP_DIR

exit $RETVAL

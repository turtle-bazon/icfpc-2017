
cd $TEMP_DIR
tar xf build.tar
rm -f build.tar

./install 2>&1
RETVAL=$?

cd /
rm -rf $TEMP_DIR

exit $RETVAL

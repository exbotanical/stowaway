#!/usr/bin/env sh
TEST_ROOT='/tmp/test_root'
rm -rf $TEST_ROOT

mkdir -p $TEST_ROOT/app{1,2,3}
mkdir -p $TEST_ROOT/app1/config
mkdir -p $TEST_ROOT/app1/.config/app1
mkdir -p $TEST_ROOT/app2/.config/app2
touch $TEST_ROOT/app1/config/file{1,2}
touch $TEST_ROOT/app1/.config/app1/file{1,2}
touch $TEST_ROOT/app2/.config/app2/file{1,2}
touch $TEST_ROOT/app3/file{1,2,3}

#!/bin/bash

# Simple example script on how to run the tracer.

set -eu

PIN_EXE="$PIN_ROOT/pin"
PIN_TOOL="../obj-intel64/aurora_tracer.so"

# Tracer will create a trace and logfile
WORKDIR="."
OUTPUT="$WORKDIR/test.trace"
LOGFILE="$WORKDIR/test.log"

# Note, PIN cannot process long paths, thus we use a TMP_DIR with short paths
TMP_DIR="/tmp/pin"
TMP_OUTPUT="$TMP_DIR/test.trace"
TMP_LOGFILE="$TMP_DIR/test.log"

TARGET_DIR="$1"
TARGET_BIN="mruby_trace"
TARGET_ARGS=""
SEED="$TARGET_DIR/seed/*"

TARGET="${TARGET_DIR}/${TARGET_BIN} ${TARGET_ARGS} ${SEED}"

mkdir -p $TMP_DIR
echo "${PIN_EXE} -t ${PIN_TOOL} -o ${TMP_OUTPUT} -logfile ${TMP_LOGFILE}  -- ${TARGET}"
time ${PIN_EXE} -t ${PIN_TOOL} -o ${TMP_OUTPUT} -logfile ${TMP_LOGFILE}  -- ${TARGET} || true

mv $TMP_OUTPUT $WORKDIR
mv $TMP_LOGFILE $WORKDIR

rm -rf "$TMP_DIR"

# Pretty-print output
./pprint.py -q -s pretty_test.trace test.trace

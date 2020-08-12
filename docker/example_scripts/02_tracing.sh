#/bin/bash

set -eu

if [ -z "$EVAL_DIR" ] || [ -z "$AURORA_GIT_DIR" ]; then
    echo "ERROR: set EVAL_DIR and AURORA_GIT_DIR env vars"
    exit 1
fi

if [ ! -f "$EVAL_DIR/pin-3.15-98253-gb56e429b1-gcc-linux/source/tools/AuroraTracer/obj-intel64/aurora_tracer.o" ]; then
    echo "Need to make obj-intel64/aurora_tracer.so first"
    exit 1
fi

mkdir -p $EVAL_DIR/traces
# requires at least python 3.6
pushd $AURORA_GIT_DIR/tracing/scripts > /dev/null
python3 tracing.py $EVAL_DIR/mruby_trace $EVAL_DIR/inputs $EVAL_DIR/traces
# extract stack and heap addr ranges from logfiles
python3 addr_ranges.py --eval_dir $EVAL_DIR $EVAL_DIR/traces
popd > /dev/null

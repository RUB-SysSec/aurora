#!/bin/bash

# run AFL
timeout 43200 $AFL_DIR/afl-fuzz -C -d -m none -i $EVAL_DIR/seed -o $AFL_WORKDIR -- $EVAL_DIR/mruby_fuzz @@

# save crashes and non-crashes
cp $AFL_WORKDIR/queue/* $EVAL_DIR/inputs/crashes
cp $AFL_WORKDIR/non_crashes/* $EVAL_DIR/inputs/non_crashes

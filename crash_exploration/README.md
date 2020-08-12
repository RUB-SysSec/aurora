# Crash exploration

## Setup
Download alf-2.52b and apply the crash_exploration.patch with
`patch -p1 < crash_exploration.patch`

## Usage
Run AFL with the desired options. Make sure to use a crashing input as seed and the -C flag to run AFL's crash exploration mode. Modified afl will then save crashing ('queue' folder) and non-crashing inputs ('non_crashes' folder) which are needed for tracing.


#!/usr/bin/python3
#pylint: disable = missing-module-docstring, no-member, redefined-outer-name

from functools import partial
from typing import List
import os
import sys
import time
import random
import logging
import zipfile
import subprocess
import multiprocessing

###############################################################################
PIN_EXE = os.environ['PIN_ROOT'] + "/pin"
PIN_TOOL = os.environ['PIN_ROOT'] + "/source/tools/AuroraTracer/obj-intel64/aurora_tracer.so"

ASAN_OPTIONS = "ASAN_OPTIONS=detect_leaks=0"

TMP_PATH = "/tmp/tm/"
PIN_TIMEOUT = 5 * 60
PARALLEL_PROCESSES = os.cpu_count()

SUBDIRS = ["crashes/", "non_crashes/"]

###############################################################################

SUCCESS = True
FAILURE = False
logger = logging.getLogger('tracing_manager') # pylint: disable=invalid-name
trace_logger = logging.getLogger('tracer') # pylint: disable=invalid-name
rng = random.SystemRandom() # pylint: disable=invalid-name

###############################################################################

# Check if input folder and output folder are passed as parameters
def preliminary_checks(target_exe: str) -> bool:
    """Checks to avoid creation of bad traces."""
    # Check if ASLR is still enabled and abort
    with open("/proc/sys/kernel/randomize_va_space", 'r') as f:
        if not "0" in f.read().strip():
            logger.critical("[!] Disable ASLR: echo 0 | sudo tee /proc/sys/kernel/randomize_va_space")
            return FAILURE
    # Check if temporary directory still exits and abort
    if os.path.exists(TMP_PATH):
        logger.critical(f"[!] Temporary directory {TMP_PATH} already exists. Backup its contents and delete it to proceed.")
        return FAILURE
    # Check if we try to trace a bash file
    file_output = str(subprocess.check_output(['file', target_exe]))
    if "Bourne-Again shell script" in file_output:
        logger.critical("[!] Target binary is a bash script")
        return FAILURE
    return SUCCESS


def zip_input(src_file: str, should_delete_original: bool = True) -> None:
    """Replaces an input by a zipped version of it."""
    zip_file = src_file + ".zip"
    file_name = os.path.basename(src_file)
    logger.debug(f"Zipping into {zip_file}")
    with zipfile.ZipFile(zip_file, 'w', compression=zipfile.ZIP_BZIP2, allowZip64=True) as f:
        f.write(src_file, arcname=file_name)
    if should_delete_original:
        os.remove(src_file)
        logger.debug(f"Deleted after zipping original: {src_file}")


def check_size(path: str) -> bool:
    """Check file size"""
    if os.stat(path).st_size == 0:
        logger.warning(f"File has size 0 => {path} is empty")
        os.remove(path)
        return FAILURE
    return SUCCESS


def check_trace_log(path: str) -> bool:
    """Check whether logfile reports Trace completed, i.e., whether trace is complete and whether multiple traces are contained."""
    log_path = os.path.join(os.path.dirname(os.path.dirname(path)), "logs", os.path.basename(path) + ".log")
    logger.debug(f"Checking logfile at {log_path} for completeness")
    with open(log_path, 'r') as logfile:
        data = logfile.read()
        count = data.count("[=] Completed trace")
        for line in data.split("\n"):
            if "[E]" in line:
                trace_logger.error(line.split("[E]")[1])
            elif "[W]" in line:
                trace_logger.warning(line.split("[W]")[1])
    if count < 1:
        logger.warning(f"Incomplete trace: {path}")
        os.remove(path)
        logger.info(f"Deleted incomplete {path}")
        return FAILURE
    if count > 1:
        logger.warning(f"Multiple traces ({count}x) in one file {path}")
        os.remove(path)
        logger.info(f"Deleted multiple-trace containing {path}")
        return FAILURE
    return SUCCESS


def trace_input(src_path: str, target_exe: str, should_zip: bool, trace_target: str) -> None:
    """Trace a directory within a given path."""
    if trace_target == "README.txt": # Skip README file
        return
    (subdir, trace_target) = trace_target.split("-", 1)
    logger.debug(f"subdir was reconstructed as '{subdir}', trace_target as {trace_target}'")
    src_file = os.path.join(src_path, subdir, trace_target)
    pin_logfile = os.path.join(TMP_PATH, "logs", trace_target + "_trace.log")
    outfile = os.path.join(TMP_PATH, subdir, trace_target + "_trace")
    if "@@" in target_exe:
        target_exe = target_exe.replace("@@", src_file)
    else:
        target_exe += f" < {src_file}"
    cmd = f"{ASAN_OPTIONS} {PIN_EXE} -t {PIN_TOOL} -o {outfile} -logfile {pin_logfile} -- {target_exe}"
    logger.debug(f"CMD: {cmd}")
    try:
        subprocess.run(cmd, shell=True, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=PIN_TIMEOUT)
    except subprocess.TimeoutExpired:
        logger.info(f"Timeout for {src_file}")
        #os.remove(src_file)
        #logger.info(f"Deleted timeouted file {src_file}")
    except subprocess.CalledProcessError as err:
        logger.debug(f"Process errored out for {cmd} with {err}")

    if check_trace_log(outfile) == SUCCESS:
        if os.path.isfile(outfile) and check_size(outfile) == SUCCESS and should_zip:
            zip_input(src_file=outfile, should_delete_original=True)


def create_dir(path: str) -> None:
    """Create new directory"""
    cmd = f"mkdir -p {path}"
    os.system(cmd)


def remove_tmp_dir() -> None:
    """Delete temporary directory if TMP_PATH appears to be safe"""
    if not(len(TMP_PATH) > 5 and TMP_PATH.startswith("/tmp/")):
        logger.critical(f"TMP PATH might be not what you expect; skipping deletion - path is {TMP_PATH}")
        return
    logger.info(f"Deleting temporary directory {TMP_PATH}")
    cmd = f"rm -rf {TMP_PATH}"
    os.system(cmd)


def cleanup(target_exe: str, save_path: str) -> None:
    """Cleanup after tracing: Kill remaining targets; move files to save_path and remove temporary directory"""
    target = os.path.basename(target_exe.split(" ", 1)[0])
    logger.info(f"killall {target}")
    cmd = f"killall -s SIGKILL {target}"
    os.system(cmd)

    start_time = time.time()
    logger.info(f"Moving files from {TMP_PATH} to {save_path}")
    cmd = f"mv {TMP_PATH}/* {save_path}"
    os.system(cmd)

    remove_tmp_dir()

    move_time = time.time()
    logger.info(f"Cleanup time: {move_time - start_time}s")


def trace_all(target_exe: str, src_path: str, save_path: str, subdirs: List[str], should_zip: bool = True) -> bool:
    """Manage parallel tracing of all files"""
    if preliminary_checks(target_exe) == FAILURE:
        return FAILURE
    logger.info(f"Using files at {src_path}")
    logger.info(f"Generating temporary directory at {TMP_PATH}")
    start_time = time.time()
    create_dir(TMP_PATH)
    create_dir(save_path)
    create_dir(os.path.join(TMP_PATH, "logs"))

    files = []
    for subdir in subdirs:
        create_dir(os.path.join(TMP_PATH, subdir))
        src_subdir = os.path.join(src_path, subdir)
        files.extend([f"{subdir}-{x}" for x in os.listdir(src_subdir)])
        create_dir(os.path.join(save_path, subdir))

    rng.shuffle(files) # shuffle files to avoid timestamp being a 'good' predicate
    logger.info(f"Processing {len(files)} files in {len(subdirs)} subdirs at {src_path}")
    before_time = time.time()
    with multiprocessing.Pool(PARALLEL_PROCESSES) as pool:
        func = partial(trace_input, src_path, target_exe, should_zip)
        pool.map(func, files)
    trace_time = time.time() - before_time
    avg_time = (PARALLEL_PROCESSES * trace_time) / len(files)
    num_files = 0
    for subdir in subdirs:
        num_files += len(os.listdir(os.path.join(TMP_PATH, subdir)))
    logger.info(f"Done processing {num_files} files in {trace_time}s (on average {avg_time}s per input)")

    logger.info(f"STATS: traced {num_files}/{len(files)} files in {trace_time}s with {PARALLEL_PROCESSES} cores for {src_path}")

    cleanup(target_exe, save_path)

    # write stats to file
    with open(os.path.join(save_path, "stats.txt"), 'w') as stats_file:
        stats_file.write(f"STATS: traced {num_files}/{len(files)} files in {trace_time}s with {PARALLEL_PROCESSES} cores for {src_path}\n")

    logger.info(f"Total execution time: {time.time() - start_time}s")
    return SUCCESS



if __name__ == "__main__":
    if len(sys.argv) < 4:
        logger.critical("Usage: ./tracing.py <PATH TO BINARY> <INPUT_FOLDER> <OUTPUT FOLDER>")
        print("Usage: ./tracing.py <PATH TO BINARY> <INPUT_FOLDER> <OUTPUT FOLDER>")
        sys.exit(1)

    ### Logging handlers
    # Create handlers
    c_handler = logging.StreamHandler()             # pylint: disable=invalid-name
    f_handler = logging.FileHandler('tracing.log')  # pylint: disable=invalid-name
    c_handler.setLevel(logging.INFO)
    f_handler.setLevel(logging.DEBUG)

    logger.setLevel(logging.DEBUG)
    trace_logger.setLevel(logging.DEBUG)

    # Create formatters and add it to handlers
    c_format = logging.Formatter('%(levelname)s: %(message)s')              # pylint: disable=invalid-name
    f_format = logging.Formatter('%(asctime)s %(levelname)s: %(message)s')  # pylint: disable=invalid-name
    c_handler.setFormatter(c_format)
    f_handler.setFormatter(f_format)

    # Add handlers to the logger
    logger.addHandler(c_handler)
    logger.addHandler(f_handler)
    trace_logger.addHandler(c_handler)
    trace_logger.addHandler(f_handler)

    if trace_all(target_exe=sys.argv[1], src_path=sys.argv[2], save_path=sys.argv[3], subdirs=SUBDIRS) == SUCCESS:
        logger.info("Finished tracing run")

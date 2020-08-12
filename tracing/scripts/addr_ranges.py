#!/usr/bin/env python3
"""Extract stack and heap address ranges from logfiles"""

from argparse import ArgumentParser
from pathlib import Path
from typing import Dict, List, Optional
import json
import sys


def dump_to_file(data: Dict[str, Dict[str, str]], path: Path) -> None:
    """Dump dict as JSON to file"""
    with open(path, "w") as fd:
        json.dump(data, fd)


def _range_to_dict(d: Dict[str, str]) -> Dict[str, Dict[str, str]]:
    """Turn stack/heap range into dict with 'start' and 'end' keys"""
    return {k : dict(zip(('start', 'end'), v.split(" - "))) for (k, v) in d.items()}


def _overapproximate(ld: List[Dict[str, Dict[str, str]]]) -> Dict[str, Dict[str, str]]:
    """extract lowest start and highest end address"""
    acc = lambda f, k, t: f([d[t][k] for d in ld if t in d])
    return {k : {'start' : acc(min, 'start', k), 'end' : acc(max, 'end', k)} for k in ld[0].keys()}


def _flatten(d: Dict[str, Dict[str, str]]) -> Dict[str, int]:
    """Flatten to one laye by concatting keys"""
    return {f"{k.lower()}_{kk}": int(vv, 16) for (k,v) in d.items() for (kk, vv) in v.items()}


def parse_logfile(logfile: Path) -> Dict[str, Dict[str, str]]:
    """Parse Stack and Heap start + end address from specified logfile"""
    with open(logfile, 'r') as fd:
        ranges: Dict[str, str] = dict(l.strip().lstrip("[*] ").split(": ") \
                        for l in fd.readlines() if l.strip() and ("Stack" in l or "Heap" in l))
    address_dict = _range_to_dict(ranges)
    # Check we didn't mess up the range's order
    for k, d in address_dict.items():
        assert int(d['start'], 16) < int(d['end'], 16), f"[{k}] Start address {d['start']} > end address {d['end']}"
    return address_dict    


def extract_stack_and_heap_address(trace_dir: Path, eval_dir: Optional[Path], exact: bool = False) -> None:
    """Extract stack and heap address ranges from some logfile"""
    logfiles = list(trace_dir.glob("./*"))
    address_dicts = list(map(parse_logfile, logfiles))
    if exact:
        unique_address_dicts = set(map(str, map(parse_logfile, logfiles)))
        assert len(unique_address_dicts) == 1, f"Found {len(unique_address_dicts)} unique address ranges overall logfiles (should be 1)"
    address_dict = _flatten(_overapproximate(list(address_dicts)))
    print(json.dumps(address_dict))
    if not eval_dir is None:
        print(f"Dumping to {eval_dir / 'addresses.json'}")
        dump_to_file(address_dict, eval_dir / 'addresses.json')


if __name__ == '__main__':
    parser = ArgumentParser(description='Extract stack and heap address ranges from logfiles') # pylint: disable=invalid-name
    parser.add_argument('trace_dir', nargs=1, help="path to traces directory")
    parser.add_argument('--eval_dir', nargs=1, action='store', default=[], help="path to evaluation directory")
    parser.add_argument('--exact', action="store_true", default=False, help="Guarantee that address range is the same for all logfiles")

    cargs = parser.parse_args() # pylint: disable=invalid-name

    trace_dir = (Path(cargs.trace_dir[0]) / "logs").resolve() # pylint: disable=invalid-name
    if not trace_dir.exists():
        print(f"Trace dir {trace_dir} does not exist. Aborting..")
        sys.exit(1)
    eval_dir = None
    if cargs.eval_dir:
        eval_dir = Path(cargs.eval_dir[0]).resolve()
    extract_stack_and_heap_address(trace_dir, eval_dir, cargs.exact)


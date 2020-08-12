#!/usr/bin/python3

import json
import argparse

parser = argparse.ArgumentParser(description='Pretty-print a trace.')
parser.add_argument('-s', "--save", help="save output additionally to file", action='store', metavar='FILENAME')
parser.add_argument('-q', "--quiet", help="don't print to stdout", action='store_true')
parser.add_argument('path', help="path to trace file")
args = parser.parse_args()

with open(args.path, 'r') as f:
    content = json.loads(f.read())
if not args.quiet:
    print(json.dumps(content, indent=4, sort_keys=True))
if args.save:
    with open(args.save, 'w') as f:
        json.dump(content, f, indent=4, sort_keys=True)


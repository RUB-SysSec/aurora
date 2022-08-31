#!/usr/bin/env bash

set -eu

# This script pulls the docker image from Dockerhub and changes the tag
# such that the convenience scripts run.sh still works as expected

# Use this *instead* of manually building the docker image

# pull image
echo "Pulling mu00d8/aurora:latest"
docker pull mu00d8/aurora:latest

# re-tag image
echo "Changing tag to aurora:latest"
docker tag mu00d8/aurora:latest aurora:latest


#!/bin/bash

docker build --build-arg USER_UID="$(id -u)" --build-arg USER_GID="$(id -g)" $@ -t aurora:latest .

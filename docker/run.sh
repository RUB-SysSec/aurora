#!/bin/bash

set -eu

IMAGE_NAME="aurora:latest"

function yes_no() {
    if [[ "$1" == "yes" || "$1" == "y" ]]; then
        return 0
    else
        return 1
    fi
}

ancestor="$(docker ps --filter="ancestor=${IMAGE_NAME}" --latest --quiet)"

if [[ ! -z "$ancestor" ]]; then
    read -p "Found running instance: $ancestor, connect?" yn
    if yes_no "$yn"; then
        cmd="docker exec -it --user "$UID:$(id -g)" $ancestor /usr/bin/bash"
        echo $cmd
        $cmd
        exit 0
    fi
    #Else; Create new container
fi


cmd="docker run --rm -it --privileged ${IMAGE_NAME} /usr/bin/bash"

echo "$cmd"
$cmd


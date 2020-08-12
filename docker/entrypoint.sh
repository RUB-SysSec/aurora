#!/bin/bash

# Add local user
# Either use the LOCAL_USER_ID if passed in at runtime or
# fallback

set -eu

if [[ -z "$LOCAL_USER_ID" ]]; then
    echo "Please set LOCAL_USER_ID"
    exit 1
fi

if [[ -z "$LOCAL_GROUP_ID" ]]; then
    echo "Please set LOCAL_GROUP_ID"
    exit 1
fi

echo "$LOCAL_USER_ID:$LOCAL_GROUP_ID"
export uid=$LOCAL_USER_ID
export gid=$LOCAL_GROUP_ID
export HOME=/home/user
export USER=user

usermod -u $LOCAL_USER_ID user
groupmod -g $LOCAL_GROUP_ID user

exec /usr/sbin/gosu user $@

#!/usr/bin/env bash

./build.sh

docker rm $(docker ps -a -q)

docker run gmpc/gmpc
#!/usr/bin/env sh

docker container ls -a | grep stowaway | awk '{ print $1 }' | xargs docker rm
docker image ls | grep stowaway | awk '{ print $3 }' | xargs docker rmi
docker build -f Dockerfile.dev -t stowaway .
docker run -it --entrypoint /bin/bash -v $(pwd):/usr/app stowaway

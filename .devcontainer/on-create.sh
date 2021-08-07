#!/bin/bash

echo "on-create start" >> ~/status

# copy grafana.db to /grafana
sudo rm -f /grafana/grafana.db
sudo cp deploy/grafanadata/grafana.db /grafana
sudo chown -R 472:472 /grafana

docker network create kind

# create local registry
docker run -d --net kind --restart=always -p "127.0.0.1:5000:5000" --name kind-registry registry:2

echo "on-create complete" >> ~/status

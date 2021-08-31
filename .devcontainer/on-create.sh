#!/bin/bash

echo "on-create start" >> ~/status

# copy grafana.db to /grafana
sudo rm -f /grafana/grafana.db
sudo cp deploy/grafanadata/grafana.db /grafana
sudo chown -R 472:0 /grafana

docker network create kind

# create local registry
#docker run -d --net kind --restart=always -p "127.0.0.1:5000:5000" --name registry registry:2
k3d registry create registry.localhost --port 5000
docker network connect kind k3d-registry.localhost

echo "on-create complete" >> ~/status

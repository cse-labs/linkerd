#!/bin/bash

echo "on-create start" >> ~/status

# copy grafana.db to /grafana
sudo rm -f /grafana/grafana.db
sudo cp deploy/grafanadata/grafana.db /grafana
sudo chown -R 472:0 /grafana

docker network create k3d

# create local registry
k3d registry create registry.localhost --port 5000
docker network connect k3d k3d-registry.localhost

echo "on-create complete" >> ~/status

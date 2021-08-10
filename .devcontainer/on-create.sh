#!/bin/bash

echo "on-create start" >> ~/status

# copy grafana.db to /grafana
sudo rm -f /grafana/grafana.db
sudo cp deploy/grafanadata/grafana.db /grafana
sudo chown -R 472:472 /grafana

docker network create kind

# create local registry
#docker run -d --net kind --restart=always -p "127.0.0.1:5000:5000" --name kind-registry registry:2

# download rio
curl -sfL https://get.rio.io | sh -

# install rust musl target
sudo apt-get install -y --no-install-recommends musl-tools
rustup target add x86_64-unknown-linux-musl

echo "on-create complete" >> ~/status

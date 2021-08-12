#!/bin/bash

echo "on-create start" >> ~/status

# copy grafana.db to /grafana
sudo rm -f /grafana/grafana.db
sudo cp deploy/grafanadata/grafana.db /grafana
sudo chown -R 472:472 /grafana

docker network create kind

# create local registry
#docker run -d --net kind --restart=always -p "127.0.0.1:5000:5000" --name registry registry:2
k3d registry create registry.localhost --port 5000
docker network connect kind k3d-registry.localhost

# push ngsa-app to local repo
docker pull ghcr.io/retaildevcrews/ngsa-app:beta
docker tag ghcr.io/retaildevcrews/ngsa-app:beta k3d-registry.localhost:5000/ngsa:local
docker push k3d-registry.localhost:5000/ngsa:local
docker rmi ghcr.io/retaildevcrews/ngsa-app:beta

# download rio
#curl -sfL https://get.rio.io | sh -

echo "on-create complete" >> ~/status

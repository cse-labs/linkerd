.PHONY: help all create delete deploy check clean app webv test reset-prometheus reset-grafana jumpbox

help :
	@echo "Usage:"
	@echo "   make all              - create a cluster and deploy the apps"
	@echo "   make create           - create a kind cluster"
	@echo "   make delete           - delete the kind cluster"
	@echo "   make deploy           - deploy the apps to the cluster"
	@echo "   make check            - check the endpoints with curl"
	@echo "   make test             - run a WebValidate test"
	@echo "   make load-test        - run a 60 second WebValidate test"
	@echo "   make clean            - delete the apps from the cluster"
	@echo "   make app              - build and deploy a local app docker image"
	@echo "   make webv             - build and deploy a local WebV docker image"
	@echo "   make reset-prometheus - reset the Prometheus volume (existing data is deleted)"
	@echo "   make reset-grafana    - reset the Grafana volume (existing data is deleted)"
	@echo "   make jumpbox          - deploy a 'jumpbox' pod"

all : delete create app

app :
	# build the local image and load into k3d
	@cd app && docker build . -t k3d-registry.localhost:5000/pickle:local
	@docker push k3d-registry.localhost:5000/pickle:local
	@kubectl apply -f deploy/pickle-local
	@kubectl get pods

delete :
	# delete the cluster (if exists)
	@# this will fail harmlessly if the cluster does not exist
	@k3d cluster delete

create :
	@# create the cluster and wait for ready
	@# this will fail harmlessly if the cluster exists
	@# default cluster name is k3d-k3s-default

	k3d cluster create --registry-use k3d-registry.localhost:5000 --config deploy/k3d/k3d.yaml

	# wait for cluster to be ready
	@kubectl wait node --for condition=ready --all --timeout=60s

check :
	# curl all of the endpoints
	@curl localhost:30088/

clean :
	# delete the deployment
	@# continue on error
	-kubectl delete -f deploy/pickle-local --ignore-not-found=true

	# show running pods
	@kubectl get po -A

### Not Working Yet
deploy :
	# deploy the app
	@# continue on most errors
	-kubectl apply -f ../deploy/ngsa-memory

	# deploy prometheus and grafana
	-kubectl apply -f ../deploy/prometheus
	-kubectl apply -f ../deploy/grafana

	# deploy fluent bit
	-kubectl create secret generic log-secrets --from-literal=WorkspaceId=dev --from-literal=SharedKey=dev
	-kubectl apply -f ../deploy/fluentbit/account.yaml
	-kubectl apply -f ../deploy/fluentbit/log.yaml
	-kubectl apply -f ../deploy/fluentbit/stdout-config.yaml
	-kubectl apply -f ../deploy/fluentbit/fluentbit-pod.yaml

	# deploy WebValidate after the app starts
	@kubectl wait pod ngsa-memory --for condition=ready --timeout=30s
	-kubectl apply -f ../deploy/webv

	# wait for the pods to start
	@kubectl wait pod -n monitoring --for condition=ready --all --timeout=30s
	@kubectl wait pod fluentb --for condition=ready --timeout=30s
	@kubectl wait pod webv --for condition=ready --timeout=30s

	# display pod status
	@kubectl get po -A | grep "default\|monitoring"

webv :
	# build the local image and load into k3d
	docker build ../../webvalidate -t webv:local
	
	k3d image import webv:local

	# display current version
	-http localhost:30088/version

	# delete / create WebValidate
	-kubectl delete -f ../deploy/webv --ignore-not-found=true
	kubectl apply -f ../deploy/webv-local
	kubectl wait pod webv --for condition=ready --timeout=30s
	@kubectl get po

	# display the current version
	@http localhost:30088/version

test :
	# run a single test
	webv --verbose --server http://localhost:30080 --files ../webv/benchmark.json

load-test :
	# use WebValidate to run a 60 second test
	webv --verbose --server http://localhost:30080 --files ../webv/benchmark.json --run-loop --sleep 100 --duration 60

reset-prometheus :
	# remove and create the /prometheus volume
	@sudo rm -rf /prometheus
	@sudo mkdir -p /prometheus
	@sudo chown -R 65534:65534 /prometheus

reset-grafana :
	# remove and copy the data to /grafana volume
	@sudo rm -rf /grafana
	@sudo mkdir -p /grafana
	@sudo cp -R ../deploy/grafanadata/grafana.db /grafana
	@sudo chown -R 472:472 /grafana

jumpbox :
	@# start a jumpbox pod
	@-kubectl delete pod jumpbox --ignore-not-found=true

	@kubectl run jumpbox --image=alpine --restart=Always -- /bin/sh -c "trap : TERM INT; sleep 9999999999d & wait"
	@kubectl wait pod jumpbox --for condition=ready --timeout=30s
	@kubectl exec jumpbox -- /bin/sh -c "apk update && apk add bash curl httpie" > /dev/null
	@kubectl exec jumpbox -- /bin/sh -c "echo \"alias ls='ls --color=auto'\" >> /root/.profile && echo \"alias ll='ls -lF'\" >> /root/.profile && echo \"alias la='ls -alF'\" >> /root/.profile && echo 'cd /root' >> /root/.profile" > /dev/null

	#
	# use kje <command>
	# kje http ngsa-memory:8080/version
	# kje bash -l

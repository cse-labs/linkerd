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

all : delete create check

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

	# deploy linkerd with monitoring
	# from https://linkerd.io/2.10/getting-started/
	@curl -sL https://run.linkerd.io/install | sh
	@linkerd install --image-pull-policy IfNotPresent | kubectl apply -f -
	@kubectl wait pod --for condition=ready --all --namespace=linkerd --timeout=10m
	@linkerd viz install | kubectl apply -f - # on-cluster metrics stack
	# to see, use: linkerd viz dashboard &
	@linkerd jaeger install | kubectl apply -f - # Jaeger collector and UI

	# deploy fluent bit
	-kubectl create secret generic log-secrets --from-literal=WorkspaceId=dev --from-literal=SharedKey=dev
	-kubectl apply -f deploy/fluentbit/account.yaml
	-kubectl apply -f deploy/fluentbit/log.yaml
	-kubectl apply -f deploy/fluentbit/stdout-config.yaml
	-kubectl apply -f deploy/fluentbit/fluentbit-pod.yaml

	# wait for the pods to start
	@kubectl wait pod fluentb --for condition=ready --timeout=60s

	# display pod status
	@kubectl get po -A

check :
	@linkerd check

	# curl /
	@curl localhost:30088/
	@echo

	# curl /v1.0
	@curl localhost:30088/v1.0

	# display pod status
	@kubectl get po -A

clean :
	# delete the deployment
	@# continue on error
	-kubectl delete -f deploy/pickle-local --ignore-not-found=true

	# show running pods
	@kubectl get po -A

app :
	docker build app/pickle -t pickle:local
	k3d image import pickle:local
	docker build app/depa -t pickle_depa:local
	k3d image import pickle_depa:local
	docker build app/depb -t pickle_depb:local
	k3d image import pickle_depb:local

deploy :
	# build the local image and load into k3d
	@kubectl apply -f deploy/app/pickle_ns.yaml
	@kubectl create -f deploy/app/pickle.yaml -n pickle
	@kubectl wait node --for condition=ready --all --timeout=30s

undeploy :
	@kubectl delete namespace pickle

webv :
	# build the local image and load into k3d
	docker build ../../webvalidate -t webv:local
	
	k3d image import webv:local

	# display current version
	-http localhost:30088/version

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

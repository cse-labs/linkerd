# Kubernetes linkerd, Rust, and grpc Codespace

Inner loop Kubernetes development, using `k3d` running in [GitHub Codespaces](https://github.com/features/codespaces), created from [Kubernetes Dev Cluster on Codespaces Template](https://github.com/retaildevcrews/kind-k3d-codespaces-template)

![License](https://img.shields.io/badge/license-MIT-green.svg)

## Overview

This project sets up a Kubernetes developer cluster using `K3d` with linkerd in a `GitHub Codespace` or local `Dev Container`.

We use this for `inner-loop` Kubernetes development. Note that it is not appropriate for production use (but is a great `Developer Experience`)

> This Codespace is tested with `zsh` and `oh-my-zsh` - it "should" work with bash ...

## Open with Codespaces

- Click the `Code` button on your repo
- Click `Open with Codespaces`
- Click `New Codespace`
- Choose the `4 core` or `8 core` option

## Build and Deploy Cluster

By default the solution will create a `kind` cluster. If you want to use [k3d](https://k3d.io/), run the make commands from the `k3d` directory

  ```bash

  # (optional) use the k3d makefile
  cd k3d

  # build the cluster
  make all

  ```

![Running Codespace](./images/RunningCodespace.png)

## Validate deployment with k9s

- From the Codespace terminal window, start `k9s`
  - Type `k9s` and press enter
  - Press `0` to select all namespaces
  - Wait for all pods to be in the `Running` state (look for the `STATUS` column)
  - Use the arrow key to select `nsga-memory` then press the `l` key to view logs from the pod
  - To go back, press the `esc` key
  - To view other deployed resources - press `shift + :` followed by the deployment type (e.g. `secret`, `services`, `deployment`, etc).
  - To exit - `:q <enter>`

![k9s](./images/k9s.png)

### Other interesting endpoints

Open [curl.http](./curl.http)

> [curl.http](./curl.http) is used in conjuction with the Visual Studio Code [REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client) extension.
>
> When you open [curl.http](./curl.http), you should see a clickable `Send Request` text above each of the URLs

![REST Client example](./images/RESTClient.png)

Clicking on `Send Request` should open a new panel in Visual Studio Code with the response from that request like so:

![REST Client example response](./images/RESTClientResponse.png)

## Jump Box

A `jump box` pod is created so that you can execute commands `in the cluster`

- use the `kj` alias
  - `kubectl exec -it jumpbox -- bash -l`
    - note: -l causes a login and processes `.profile`
    - note: `sh -l` will work, but the results will not be displayed in the terminal due to a bug

- use the `kje` alias
  - `kubectl exec -it jumpbox --`
- example
  - run http against the ClusterIP
    - `kje http ngsa-memory:8080/version`

## View Fluent Bit Logs

- Start `k9s` from the Codespace terminal
- Select `fluentb` and press `enter`
- Press `enter` again to see the logs
- Press `s` to Toggle AutoScroll
- Press `w` to Toggle Wrap
- Review logs that will be sent to Log Analytics when configured

## Next Steps

> [Makefile](./Makefile) is a good place to start exploring

## Trademarks

This project may contain trademarks or logos for projects, products, or services.

Authorized use of Microsoft trademarks or logos is subject to and must follow [Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).

Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.

Any use of third-party trademarks or logos are subject to those third-party's policies.

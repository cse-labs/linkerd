# Note: You can use any Debian/Ubuntu base image
FROM ghcr.io/retaildevcrews/kind-rust:latest

# user args
# some base images require specific values
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# [Optional] Uncomment this section to install additional OS packages.
# alternatively, you can add commands to on-create.sh and/or post-create.sh
# on-create runs before the Codespaces UI appears
# post-create runs in the background after the UI appears
# RUN apt-get -y install --no-install-recommends <your-package-list-here>

# upgrade packages
# you can improve startup time by runing in post-create.sh
# RUN apt upgrade -y

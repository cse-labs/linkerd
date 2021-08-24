#!/bin/bash

echo "post-start start" >> ~/status

rm -f /workspaces/linkerd/app/rust/signer/keys/pickle.key
echo "$PICKLE_PRIVATE_KEY" >> /workspaces/linkerd/app/rust/signer/keys/pickle.key

echo "post-start complete" >> ~/status

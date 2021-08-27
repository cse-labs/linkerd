#!/bin/bash

echo "post-start start" >> ~/status

rm -f /workspaces/linkerd/app/rust/signer/keys/pickle_key.der
echo "$PICKLE_PRIVATE_KEY" | openssl base64 -d >> /workspaces/linkerd/app/rust/signer/keys/pickle_key.der

echo "post-start complete" >> ~/status

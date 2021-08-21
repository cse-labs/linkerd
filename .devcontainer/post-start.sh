#!/bin/bash

echo "post-start start" >> ~/status

rm -f ../app/rust/signer/keys/pickle.key
echo "$PICKLE_PRIVATE_KEY" >> ../app/rust/signer/keys/pickle.key

echo "post-start complete" >> ~/status

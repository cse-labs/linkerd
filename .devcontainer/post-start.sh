#!/bin/bash

echo "post-start start" >> ~/status

rm -f ../app/depb/keys/pickle.key
echo "$PICKLE_PRIVATE_KEY" >> ../app/depb/keys/pickle.key

echo "post-start complete" >> ~/status

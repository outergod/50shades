#!/bin/bash -

echo  "insert 10000 test docs..."
for i in {0..10000}
do
  curl --location --request POST 'http://localhost:9200/testidx/_doc/?pretty' \
  --header 'Content-Type: application/json' \
  --data-raw "{
      \"msg\": \"msg$i\"
  }"
done
echo "done."


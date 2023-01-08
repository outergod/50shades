#!/bin/bash -

echo "creating raw input on tcp/5555..."
curl -u admin:testsecret -H "X-Requested-By: initscript" --header "Content-Type: application/json" \
  --request POST \
  --data '{ "title": "rrr", "global": true, "type": "org.graylog2.inputs.raw.tcp.RawTCPInput", "configuration": { "tcp_keepalive": false,  "use_null_delimiter": false,  "tls_client_auth_cert_file": "",  "bind_address": "0.0.0.0",  "tls_cert_file": "",  "port": 5555,  "tls_key_file": "",  "tls_enable": false,  "tls_key_password": "",  "tls_client_auth": "disabled",  "charset_name": "UTF-8"} }' \
  http://localhost:9000/api/system/inputs 
echo "done."

echo  "generating 10000 test messages..."
for i in {0..10000}
do
  echo "Msg: $i" | nc -N localhost 5555
done
echo "done."


This meant to be a simple docker based test environment template for graylog (latest configured with opensearch instead of elasticsearch) 
*Prerequistes* are: `docker`/`docker-compose`, `bash` and `nc`

# Steps to start
1. cd into test-env graylog directory
1. `docker compose up`
1. run `./create_input_and_insert_data.sh` which create a raw tcp input in graylog listening on port 5555 and pushes 10000 sample messages into it via nc

# tests

```sh
curl -X POST http://127.0.0.1:3000/api/hubs 

# {"id":"IiJrDLv7pi","url":"http://127.0.0.1:3000/api/hubs/IiJrDLv7pi","text_url":"http://127.0.0.1:3000/api/hubs/IiJrDLv7pi/text","expires_at":"2025-08-14T10:58:51.018971+00:00"}%

curl http://127.0.0.1:3000/api/hubs/IiJrDLv7pi
# {"id":"IiJrDLv7pi","content":"yo it works","created_at":"2025-08-13T10:58:51.018971Z","expires_at":"2025-08-14T10:58:51.018971Z"}%  

curl -X PUT -H "Content-Type: text/plain" --data "update it!!!!" http://127.0.0.1:3000/api/hubs/IiJrDLv7pi/text

curl http://127.0.0.1:3000/api/hubs/IiJrDLv7pi  
# {"id":"IiJrDLv7pi","content":"update it!!!!","created_at":"2025-08-13T10:58:51.018971Z","expires_at":"2025-08-14T10:58:51.018971Z"}%  



# ....

# with redis and MinIO

REDIS_URL=redis://127.0.0.1/ \
AWS_ACCESS_KEY_ID=minioadmin \
AWS_SECRET_ACCESS_KEY=minioadmin \
cargo run

curl -X POST http://127.0.0.1:3000/api/hubs
curl http://127.0.0.1:3000/api/hubs/x6VpgDikq9
curl -X POST -F "file=@test.txt" http://127.0.0.1:3000/api/hubs/x6VpgDikq9/files
# curl -X POST -F "file=@test.txt" http://127.0.0.1:3000/api/hubs/{YOUR_HUB_ID}/files

# `GO to http://127.0.0.1:9001/browser/ephemeral to see the 


```

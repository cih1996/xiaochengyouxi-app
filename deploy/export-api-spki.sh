set -eu
sudo openssl x509 -in /etc/nginx/ssl/orange-api.lingzidev.com/fullchain.pem -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256 -binary | openssl base64 -A
printf '\n'

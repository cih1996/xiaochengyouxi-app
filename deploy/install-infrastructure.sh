set -eu
install -d -m 700 /home/ubuntu/orange-private
tar -xzf /home/ubuntu/orange-private.tar.gz -C /home/ubuntu/orange-private
sudo install -m 600 /home/ubuntu/orange-private/server.env /etc/orange-rewards/server.env
for host in orange.lingzidev.com orange-api.lingzidev.com; do
  sudo install -m 644 "/home/ubuntu/orange-private/tls/$host/fullchain.pem" "/etc/nginx/ssl/$host/fullchain.pem"
  sudo install -m 600 "/home/ubuntu/orange-private/tls/$host/privkey.pem" "/etc/nginx/ssl/$host/privkey.pem"
done
sudo install -m 644 /home/ubuntu/orange-service /etc/systemd/system/orange-rewards.service
sudo install -m 644 /home/ubuntu/orange-nginx /etc/nginx/sites-available/orange-rewards
sudo ln -sfn /etc/nginx/sites-available/orange-rewards /etc/nginx/sites-enabled/orange-rewards
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl daemon-reload
sudo systemctl enable nginx orange-rewards
sudo systemctl reload nginx
rm -rf -- /home/ubuntu/orange-private
rm -f -- /home/ubuntu/orange-private.tar.gz
printf 'TLS and systemd configuration installed.\n'

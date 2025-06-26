the code for my blog, [itsjuneti.me](https://itsjuneti.me) (and its new address, [june.cat](https://june.cat). hope y'all enjoy it, and please tell me if you find any vulnerabilities instead of exploiting them :)

## setup

here's how I set it up! for my reference and for yours, i guess

1. install postgres and nginx
2. enable their services
3. create admin user:

```bash
useradd server_admin;
mkdir -p /home/server_admin/server_files/assets
chown -R server_admin:server_admin /home/server_admin
```

4. Connect to postgresql with `sudo -u postgres psql` and execute something like:

```sql
CREATE DATABASE server_database;
CREATE USER server_admin WITH ENCRYPTED PASSWORD 'password';
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO server_admin;
ALTER DATABASE server_database OWNER TO server_admin;
```

you may need to also change the auth method for 127.0.0.1/32 in pg_hba.conf (exec `SHOW hba_file` in sql) to `password` instead of `ident` to get it to work

5. Append the following to /etc/sudoers:

```
# Allow server_admin to restart the server
server_admin ALL=NOPASSWD: /usr/bin/systemctl stop itsjunetime.service
server_admin ALL=NOPASSWD: /usr/bin/systemctl start itsjunetime.service
server_admin ALL=NOPASSWD: /usr/bin/systemctl daemon-reload
server_admin ALL=NOPASSWD: /usr/sbin/nginx -s reload
```

6. Copy your tls crt and keys to wherever you want on the system

8. Write the following to `/etc/systemd/system/itsjunetime.service`:

```systemd
[Unit]
Description=itsjuneti.me blog
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
User=server_admin
ExecStart=/home/server_admin/server_files/backend
WorkingDirectory=/home/server_admin/server_files/
Environment=LEPTOS_SITE_ROOT=/home/server_admin/server_files LEPTOS_SITE_PKG_DIR=pkg LEPTOS_SITE_ADDR=0.0.0.0:443
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

9. Write something like the following to `/home/server_admin/server_files/.env`:

```env
DATABASE_URL="postgres://server_admin:password@127.0.0.1/server_database"
BASE_PASSWORD="password"
BASE_USERNAME="june"
ASSET_DIR="/home/server_admin/server_files/assets"
CERT_FILE="/path/to/tls/cert/chain.crt"
KEY_FILE="/path/to/tls/private/key.key"
```

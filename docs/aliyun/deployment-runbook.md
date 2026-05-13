# Aliyun Deployment Runbook

This runbook deploys the independent `word-admin` backend and restores the frozen local PostgreSQL data onto an Aliyun ECS.

## Recommended First ECS

Start with:

- ECS: 2 vCPU / 4 GB RAM
- Disk: 40 GB system disk minimum, SSD/ESSD preferred
- OS: Ubuntu 22.04 LTS or Debian 12
- Security group inbound:
  - 22 SSH, restricted to your IP if possible
  - 80 HTTP
  - 443 HTTPS
- Do not expose PostgreSQL port 5432 publicly.

2 vCPU / 2 GB can run the app, but 4 GB gives more room for PostgreSQL, Node, logs, backups, and admin UI.

## Server Packages

Install:

```bash
sudo apt update
sudo apt install -y postgresql postgresql-contrib nginx certbot python3-certbot-nginx git curl
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
sudo npm install -g pm2
```

## Database Setup

Create a database and least-privilege app user:

```bash
sudo -u postgres psql
```

```sql
create user word_admin with encrypted password '<strong-password>';
create database word_admin_prod owner word_admin;
\q
```

Restore the frozen dump:

```bash
pg_restore \
  --dbname=postgres://word_admin:<strong-password>@127.0.0.1:5432/word_admin_prod \
  /opt/word/backups/word_admin_dev_2026-05-07_aliyun_freeze.dump
```

## Backend Deploy

Place the backend at:

```bash
/opt/word/word-admin
```

Install dependencies:

```bash
cd /opt/word/word-admin
npm ci --omit=dev
```

Create environment file:

```bash
sudo tee /etc/word-admin.env >/dev/null <<'EOF'
DATABASE_URL=postgres://word_admin:<strong-password>@127.0.0.1:5432/word_admin_prod
PORT=8787
HOST=127.0.0.1
EOF
```

Start with PM2:

```bash
cd /opt/word/word-admin
set -a
. /etc/word-admin.env
set +a
pm2 start src/server.mjs --name word-admin
pm2 save
pm2 startup
```

## Nginx Reverse Proxy

Example for `api.example.com`:

```nginx
server {
    listen 80;
    server_name api.example.com;

    client_max_body_size 1m;

    location / {
        proxy_pass http://127.0.0.1:8787;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable HTTPS:

```bash
sudo nginx -t
sudo systemctl reload nginx
sudo certbot --nginx -d api.example.com
```

## Post-Deploy Verification

Health:

```bash
curl https://api.example.com/health
```

Expected:

```json
{"ok":true,"service":"@word/aliyun-backend","storeMode":"postgres"}
```

Backend checks on server:

```bash
cd /opt/word/word-admin
set -a
. /etc/word-admin.env
set +a
npm test
npm run test:postgres
```

Database sanity:

```bash
psql "$DATABASE_URL" -c "select count(*) from public.users;"
psql "$DATABASE_URL" -c "select count(*) from public.report_snapshots;"
psql "$DATABASE_URL" -c "select max(error_count) from public.wrong_word_entries;"
```

Expected for the current freeze:

- users: 2
- report snapshots: 0
- max wrong word error count: 5

## Mobile Cutover

After HTTPS is ready:

1. Change the mobile Word Admin local backend URL from LAN IP to `https://api.example.com`.
2. Build a release APK.
3. Install on phone.
4. Log in with a test account first.
5. Verify:
   - login succeeds
   - wrong words restore without inflated counts
   - reports do not show thousands of questions
   - AI passages restore
   - one new study session uploads and appears in PostgreSQL

## Backup Policy

Minimum first policy:

- Daily `pg_dump --format=custom`.
- Keep 7 daily backups.
- Copy backups off the ECS disk when possible.
- Take a manual backup before every app/backend update.

Example:

```bash
mkdir -p /opt/word/backups
pg_dump --format=custom "$DATABASE_URL" \
  --file="/opt/word/backups/word_admin_prod_$(date +%F_%H%M%S).dump"
find /opt/word/backups -name 'word_admin_prod_*.dump' -mtime +7 -delete
```

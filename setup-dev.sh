#!/usr/bin/env sh
# Creates local dev config files that are gitignored.
set -e

if [ ! -f admin-token.json ]; then
  cat > admin-token.json <<'EOF'
{
  "token": "apiv3_dev-local-token",
  "name": "_admin",
  "description": "Local dev admin token — do not use in production"
}
EOF
  chmod 600 admin-token.json
  echo "Created admin-token.json"
else
  echo "admin-token.json already exists, skipping."
fi

mkdir -p ~/.influxdb3/data
echo "Done. Run 'docker compose up' to start."

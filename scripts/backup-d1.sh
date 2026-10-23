#!/usr/bin/env bash
set -euo pipefail
umask 077
: "${CLOUDFLARE_API_TOKEN:?Missing Cloudflare credential}"
: "${CLOUDFLARE_ACCOUNT_ID:?Missing Cloudflare account}"
: "${ENVIRONMENT:?Missing environment}"
case "$ENVIRONMENT" in
  production) export D1_DATABASE_ID="${D1_DATABASE_ID_PRODUCTION:?Missing production database ID}" ;;
  dev) export D1_DATABASE_ID="${D1_DATABASE_ID_DEV:?Missing dev database ID}" ;;
  *) exit 1 ;;
esac
test ! -e backup-output
mkdir backup-output
trap 'rm -f backup.sql backup.sql.gz backup-restore.sqlite' EXIT
openssl x509 -in docs/backup-recipient.pem -checkend 2592000 -noout
database_name="$(wrangler d1 list --json | python3 -c '
import json,os,sys
matches=[x["name"] for x in json.load(sys.stdin) if x["uuid"]==os.environ["D1_DATABASE_ID"]]
assert len(matches)==1,"Expected exactly one matching D1 database"
print(matches[0])')"
export_ok=false
for attempt in 1 2 3; do
  rm -f backup.sql
  if timeout 240 wrangler d1 export "$database_name" --remote --output=backup.sql; then
    test -s backup.sql
    export_ok=true
    break
  fi
  echo "D1 export attempt $attempt failed" >&2
  if test "$attempt" -lt 3; then sleep $((attempt * 30)); fi
done
test "$export_ok" = true
python3 scripts/validate-backup.py backup.sql backup-restore.sqlite > backup-output/manifest.json
gzip -c backup.sql > backup.sql.gz
gzip -t backup.sql.gz
filename="warden-${ENVIRONMENT}-$(date -u +%Y%m%dT%H%M%SZ).sql.gz.cms"
openssl cms -encrypt -binary -aes-256-cbc -outform DER \
  -in backup.sql.gz -out "backup-output/$filename" docs/backup-recipient.pem
test -s "backup-output/$filename"
(cd backup-output && sha256sum "$filename" manifest.json > SHA256SUMS)
echo "Export restore-check passed; encrypted backup ready for required upload."

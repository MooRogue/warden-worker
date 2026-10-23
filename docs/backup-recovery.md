# Encrypted D1 backups

The daily backup exports the selected database with three bounded attempts,
restores the SQL into disposable SQLite and checks integrity, compresses it,
encrypts it to the public certificate in `backup-recipient.pem`, then **requires**
a successful GitHub artifact upload. Artifacts are retained for 30 days. A green
run must include a nonempty artifact ID; an export without upload is not success.
This replaces the formerly unconfigured S3/WebDAV workflow. No destination secrets
are required for GitHub artifacts. Application deployment and database migrations
are excluded for backup-only commits.

Private recovery key (not committed and never available to GitHub):
`/mnt/backups/infra.backup/warden-worker/recovery-20261023/recovery-private.pem`.
Preserve this key independently before removing infra backups; loss of the key
means loss of access to all backups encrypted to this certificate. Rotate the
public certificate before its ten-year expiry, retaining every older private key.
An additional root-only local recovery-key copy is at
`/var/lib/warden-backup-recovery/recovery-private-20261023.pem` on Overwatch, so a
Synology storage failure alone does not remove the only key.

Download the selected `warden-d1-production-RUN_ID` artifact into a private local
directory and unpack it. Verify `sha256sum -c SHA256SUMS`. Then decrypt locally:

```sh
openssl cms -decrypt -binary -inform DER -in BACKUP.sql.gz.cms \
  -inkey /mnt/backups/infra.backup/warden-worker/recovery-20261023/recovery-private.pem \
  -out restore.sql.gz
gzip -t restore.sql.gz
gzip -dc restore.sql.gz > restore.sql
python3 scripts/validate-backup.py restore.sql restore.sqlite
```

Run these commands with access to the protected key (normally sudo), restrictive
umask 077, and remove plaintext test outputs afterward. Local restore verification
does not modify the production database. A real production restore requires
separate explicit authorization. GitHub artifact retention is not proof of a
second independently stored backup; copy verified encrypted artifacts to infra
backup storage when checking recovery.

## First end-to-end verification

Run `39949580404` on 2026-10-23 created artifact `12499174102` successfully.
The downloaded archive passed SHA-256 verification, CMS decryption, gzip checks,
and a disposable SQLite restore with 13 tables and `quick_check=ok`. SQL size was
34,738 bytes; ciphertext size was 11,790 bytes. The local restore matched the
workflow validation manifest. No application deployment or production restore
occurred. Plaintext test outputs were removed. Encrypted copy and recovery proof:
`/mnt/backups/infra.backup/warden-worker/recovery-20261023/verified-39949580404/`.

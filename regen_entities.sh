#!/usr/bin/env bash
set -xe

TEMPDB=$(mktemp /tmp/seaorm-migrate-XXXXXXX.sqlite3)
export DATABASE_URL="sqlite://$TEMPDB"

pushd notifico-app
touch "$TEMPDB"
sea-orm-cli migrate -d migration up
sea-orm-cli generate entity -o src/entity --ignore-tables notifico_migrations
rm "$TEMPDB"
popd

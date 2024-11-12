#!/usr/bin/env bash
set -xe

TEMPDB=$(mktemp /tmp/seaorm-migrate-XXXXXXX.sqlite3)
export DATABASE_URL="sqlite://$TEMPDB"

pushd notifico-subscription
touch "$TEMPDB"
sea-orm-cli migrate -d migration up
sea-orm-cli generate entity -o src/entity --ignore-tables subscription_migrations
rm "$TEMPDB"
popd

pushd notifico-dbpipeline
touch "$TEMPDB"
sea-orm-cli migrate -d migration up
sea-orm-cli generate entity -o src/entity --ignore-tables pipeline_migrations
rm "$TEMPDB"
popd

pushd notifico-project
touch "$TEMPDB"
sea-orm-cli migrate -d migration up
sea-orm-cli generate entity -o src/entity --ignore-tables project_migrations
rm "$TEMPDB"
popd

pushd notifico-template
touch "$TEMPDB"
sea-orm-cli migrate -d migration up
sea-orm-cli generate entity -o src/entity --ignore-tables template_migrations
rm "$TEMPDB"
popd

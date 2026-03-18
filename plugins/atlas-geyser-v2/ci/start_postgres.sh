#!/usr/bin/env bash

set -e
sudo systemctl start postgresql
PGPASSWORD=x1 psql -U x1 -h localhost -d x1 -f scripts/create_schema.sql

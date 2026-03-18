#!/usr/bin/env bash

set -ex

sudo apt-get update
sudo apt-get install -y postgresql

sudo systemctl start postgresql
sudo -u postgres createuser -s x1
sudo -u postgres createdb -O x1 x1
sudo -u postgres psql -c "ALTER USER x1 WITH PASSWORD 'x1';"
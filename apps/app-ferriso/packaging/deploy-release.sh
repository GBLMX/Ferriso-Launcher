#!/bin/sh
# Receive one staged release from CI, then republish channel manifests from
# the local mirror. The deploy key is restricted to this command.
set -eu
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
tar --no-same-owner -C /var/www/ferriso-updates -xzf -
exec /usr/local/sbin/ferriso-update-sync --local

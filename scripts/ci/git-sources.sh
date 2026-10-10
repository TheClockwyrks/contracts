#!/usr/bin/env bash
# Let a pipeline run fetch a dependency on another repository from Azure.
#
#   SYSTEM_ACCESSTOKEN=... scripts/ci/git-sources.sh
#
# Every manifest names a dependency on another repository of The Test Cabinet
# by its public source, the GitHub mirror. A pipeline run holds no GitHub
# credential, and the mirrors are private while the project is; what the run
# does hold is the job's access token for Azure DevOps, where the same
# repositories live. So each public source is rewritten to the Azure
# repository over https, and the token is sent as the authorization header on
# every request to dev.azure.com. Cargo fetches through the git command line
# (`.cargo/config.toml`), which is what reads these settings.
#
# The pairs below are written by the repository kit from its one table of
# repositories, so `scripts/repos/render.py --update` in the superrepo is what
# changes them. Every repository is listed rather than the few this one may
# depend on, so a new edge needs no re-render to fetch.
set -euo pipefail

: "${SYSTEM_ACCESSTOKEN:?git-sources: SYSTEM_ACCESSTOKEN must carry the access token of the job}"

git config --global "http.https://dev.azure.com/.extraheader" \
	"AUTHORIZATION: bearer ${SYSTEM_ACCESSTOKEN}"

count=0
while read -r azure public; do
	[ -n "$azure" ] || continue
	git config --global "url.${azure}.insteadOf" "$public"
	count=$((count + 1))
done <<'SOURCES'
https://dev.azure.com/genyume/the-test-cabinet/_git/contracts https://github.com/TheClockwyrks/contracts
https://dev.azure.com/genyume/the-test-cabinet/_git/engines https://github.com/TheClockwyrks/engines
https://dev.azure.com/genyume/the-test-cabinet/_git/gg https://github.com/TheClockwyrks/gg
https://dev.azure.com/genyume/the-test-cabinet/_git/platform https://github.com/TheClockwyrks/platform
https://dev.azure.com/genyume/the-test-cabinet/_git/web https://github.com/TheClockwyrks/web
https://dev.azure.com/genyume/the-test-cabinet/_git/the-spec-cabinet https://github.com/TheClockwyrks/the-spec-cabinet
SOURCES

echo "git-sources: ${count} public sources fetch from Azure"

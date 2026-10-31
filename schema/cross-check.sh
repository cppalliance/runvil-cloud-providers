#!/usr/bin/env bash
# The sheet-schema cross-check.
#
# This repository derives its own sheet schema from its own copy of the sheet
# types. The Gateway derives its schema from its copy. Those two derivations
# are the two halves of the compatibility rule, and the `$defs` objects must
# agree: the sheet this repository publishes is the sheet a shipped Gateway
# reads.
#
# The pin names a Gateway commit and the sha256 of its committed schema, so a
# rewrite of that file on the Gateway side cannot silently change what this
# check compares against. Only `$defs` is compared, because the two wrappers
# may differ in `title` and key order.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pin="$root/schema/pin.json"
local_schema="$root/schema/sheet.schema.json"

for tool in jq curl sha256sum; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "::error::the cross-check needs $tool, which is not on PATH"
    exit 1
  fi
done

repository="$(jq -r '.repository' "$pin")"
commit="$(jq -r '.commit' "$pin")"
path="$(jq -r '.path' "$pin")"
want_sha="$(jq -r '.sha256' "$pin")"
kind="$(jq -r '.kind' "$pin")"

if [ "$kind" != "sheet-schema" ]; then
  echo "::error::schema/pin.json pins '$kind', not the sheet schema"
  exit 1
fi

url="https://raw.githubusercontent.com/$repository/$commit/$path"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "fetching $url"
if ! curl -sSL --fail --retry 3 -o "$work/pinned.json" "$url"; then
  echo "::error::could not fetch the pinned schema from $url"
  exit 1
fi

got_sha="$(sha256sum "$work/pinned.json" | cut -d' ' -f1)"
if [ "$got_sha" != "$want_sha" ]; then
  echo "::error::$path at $commit hashes to $got_sha, but schema/pin.json names $want_sha"
  exit 1
fi
echo "pinned schema matches its sha256"

jq -S '.["$defs"] // {}' "$work/pinned.json" > "$work/pinned.defs.json"
jq -S '.["$defs"] // {}' "$local_schema" > "$work/local.defs.json"

if cmp -s "$work/pinned.defs.json" "$work/local.defs.json"; then
  count="$(jq 'keys | length' "$work/local.defs.json")"
  echo "the two \$defs objects match ($count definitions)"
  exit 0
fi

# Name every definition that differs, so the failure says what to change.
status=0
while IFS= read -r name; do
  jq -S --arg name "$name" '.[$name] // null' "$work/pinned.defs.json" > "$work/pinned.one.json"
  jq -S --arg name "$name" '.[$name] // null' "$work/local.defs.json" > "$work/local.one.json"
  if cmp -s "$work/pinned.one.json" "$work/local.one.json"; then
    continue
  fi
  status=1
  # Each one-definition file was written by jq, so it ends with a newline
  # and the whole document is the single word `null` when the definition is
  # absent. `-q` alone would also match `null` as a prefix of a larger
  # document, hence the `-x` on a lone line.
  if [ "$(tr -d ' \r\n' < "$work/pinned.one.json")" = "null" ]; then
    echo "::error::the definition \`$name\` exists here and not in the Gateway"
  elif [ "$(tr -d ' \r\n' < "$work/local.one.json")" = "null" ]; then
    echo "::error::the definition \`$name\` exists in the Gateway and not here"
  else
    echo "::error::the definition \`$name\` differs between the two schemas"
    echo "--- Gateway, $repository@$commit"
    cat "$work/pinned.one.json"
    echo "--- this repository"
    cat "$work/local.one.json"
  fi
done < <(
  # `tr -d '\r'` because the Windows jq build writes CRLF, and a name
  # carrying a stray carriage return would look up nothing and compare
  # equal to nothing.
  jq -r 'keys[]' "$work/pinned.defs.json" "$work/local.defs.json" | tr -d '\r' | sort -u
)

exit "$status"

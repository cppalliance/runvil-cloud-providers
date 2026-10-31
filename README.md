# runvil-cloud-providers

[![models workflow](https://github.com/cppalliance/runvil-cloud-providers/actions/workflows/models.yml/badge.svg)](https://github.com/cppalliance/runvil-cloud-providers/actions/workflows/models.yml)
[![models release](https://img.shields.io/github/v/release/cppalliance/runvil-cloud-providers?display_name=tag&label=models)](https://github.com/cppalliance/runvil-cloud-providers/releases/tag/models)
[![download cloud-provider-models.json](https://img.shields.io/badge/download-cloud--provider--models.json-blue)](https://github.com/cppalliance/runvil-cloud-providers/releases/download/models/cloud-provider-models.json)

This repository publishes the Runvil cloud-provider model sheet as a rolling GitHub release. The sheet always lives at one stable URL:

```
https://github.com/cppalliance/runvil-cloud-providers/releases/download/models/cloud-provider-models.json
```

## What the sheet is

`cloud-provider-models.json` is a catalog of the models offered by 23 remote AI cloud providers. It is a JSON envelope with a `schema_version`, a `generated_at` timestamp, and a `providers` map keyed by provider name, where each entry carries the provider's current model list along with per-provider status and history.

## Why Runvil downloads it

Runvil periodically fetches this file so its provider menus reflect what each cloud service actually offers right now, without shipping a new build for every model launch. The fetch is an anonymous GET of a static public file: nothing is uploaded, no request identifies the user, and the sheet itself contains no API keys or account data of any kind. Users who never use cloud providers can ignore the check or disable it entirely.

## How it is built

Once a week (and on demand via workflow dispatch), the `models` workflow in this repository builds the `shared-cloud-providers` sheet binary from this repository and runs it with the provider API keys held in this repository's GitHub secrets. The binary queries every provider, merges the results with the previously published sheet, and writes a fresh `cloud-provider-models.json`. The workflow then scans the sheet to prove no secret value leaked into it, compares the per-provider model arrays against the published asset, and only when the models actually changed overwrites the release asset in place - which is what keeps the download URL above stable forever.

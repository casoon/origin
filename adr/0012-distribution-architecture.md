# ADR-0012  Distribution Architecture

Status:   Superseded by ADR-0030
Date:     2026-08-23

## Context

Signing, notarization and updater wiring are re-solved per project and usually
under time pressure shortly before a release.

## Decision (draft, Phase 5)

Release is tag-driven and identical for every product:
`tag → CI → tests → build → sign → package → release`.
Channels: `stable`, `beta`, `nightly`. Updates are signature-verified before install.

## Consequences

- Open until Phase 5; requires signing identities and secrets to be provisioned.
- Products describe distribution in the manifest, not in bespoke CI scripts.

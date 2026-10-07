# @platform/release

Build, sign and publish Desktop, CLI and daemon as one distribution.

## Recipes

The `just` recipes check, build and pack a distribution, and `rehearse` downloads, stages and activates a packed archive with throwaway signing keys.

```sh
just @platform/release/check
just @platform/release/lint
just @platform/release/format-check
just @platform/release/test
just @platform/release/build
just @platform/release/pack
just @platform/release/installer
just @platform/release/rehearse VERSION ARCHIVE
```

## Publication

`release.yml` builds, publishes and deploys one channel per run: nightly from main every night, stable on a pushed `v<version>` tag after approval, either on a manual dispatch.

```yaml
jobs:
    prepare:
    check:
        needs: [prepare]
    build:
        needs: [prepare, check]
    sign:
        needs: [prepare, build]
    build-installers:
        needs: [prepare, build, sign]
    publish:
        needs: [prepare, sign, build-installers]
    deploy:
        needs: [prepare, publish]
    drift:
        needs: [prepare]
    renew:
        needs: [prepare]
    check-expiry:
        needs: [prepare, publish, renew]
    verify:
        needs: [prepare, publish]
```

## Platforms

Each released target is one `Platform` row in `distribution/platform.ts`, which the workflow matrices, the build, the signing step and the installers read.

```ts
{
    target: "aarch64-apple-darwin",
    runner: "macos-15",
    runtime: "bun-darwin-arm64",
    system: "darwin",
    architecture: "arm64",
    bundle: APPLE_BUNDLE,
    signer: "apple",
    installers: [UNIVERSAL_DMG],
}
```

## Download layout

`PUBLIC_PATH` admits the TUF metadata, content-addressed targets, `downloads.json` and `install` each channel publishes below `download.destack.sh` from its own R2 bucket.

```sh
curl https://download.destack.sh/stable/metadata/timestamp.json
curl https://download.destack.sh/nightly/downloads.json
```

## Channels

`Release.channel` reads a version's channel, each channel with its own application title, identifier, command, icons and repository.

```ts
import { Release } from "@destack/update/release";

new Release("2026.10.1", "aarch64-apple-darwin").channel; // "stable"
new Release("2026.10.412-nightly.1", "aarch64-apple-darwin").channel; // "nightly"
```

## Downloads

The `install` script installs the Linux archive, and the first launch of the macOS DMG's application registers the CLI and the daemon.

```sh
curl -fsSL https://download.destack.sh/stable/install | sh
```

## Keys

The root role signs with two of three YubiKeys and a physical touch, and `readReleaseKeys` reads one environment's Ed25519 targets, snapshot and timestamp keys from `DESTACK_RELEASE_KEYS`.

```ts
const { targets, snapshot, timestamp } = await readReleaseKeys();
```

### Expiry

`expiry.ts` fails once root is within 90 days of expiring, targets within 30, and snapshot and timestamp within 3.

```sh
DESTACK_RELEASE_CHANNEL=stable bun run @platform/release/src/repository/expiry.ts
```

## Environments

Each GitHub environment holds only the secrets and variables its jobs sign, upload and deploy with.

```yaml
release-{channel}:
    secrets:
        [
            DESTACK_TARGETS_KEY,
            DESTACK_SNAPSHOT_KEY,
            DESTACK_TIMESTAMP_KEY,
            CLOUDFLARE_RELEASE_ACCESS_KEY_ID,
            CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY,
            APPLE_CERTIFICATE,
            APPLE_CERTIFICATE_PASSWORD,
            APPLE_NOTARY_KEY,
        ]
    variables: [APPLE_SIGNING_IDENTITY, APPLE_NOTARY_KEY_ID, APPLE_NOTARY_ISSUER]
renewal-{channel}:
    secrets: [DESTACK_SNAPSHOT_KEY, DESTACK_TIMESTAMP_KEY]
production, development, drift:
    secrets: [CLOUDFLARE_COMPANY_API_TOKEN, PLANETSCALE_SERVICE_TOKEN_ID, PLANETSCALE_SERVICE_TOKEN]
build:
    secrets: []
```

## Ceremony

`enroll.ts` enrolls a YubiKey, and `ceremony.ts` prepares a root, signs it with two YubiKeys and verifies it.

```sh
brew install ykman opensc yubico-piv-tool age
export BW_SESSION="$(bw unlock --raw)"

bun run @platform/release/src/key/enroll.ts SERIAL /absolute/ceremony/A
bun run @platform/release/src/key/rehearse.ts /absolute/ceremony/A/root.pem SERIAL /opt/homebrew/lib/libykcs11.dylib
bun run @platform/release/src/key/generate.ts /absolute/private/stable

bun run @platform/release/src/key/ceremony.ts prepare \
    /absolute/private/stable \
    /absolute/ceremony/A/root.pem /absolute/ceremony/B/root.pem /absolute/ceremony/C/root.pem \
    EXPIRY /absolute/ceremony/1.unsigned.json
bun run @platform/release/src/key/ceremony.ts sign \
    /absolute/ceremony/1.unsigned.json /absolute/ceremony/A/root.pem SERIAL_A \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.A.json
bun run @platform/release/src/key/ceremony.ts sign \
    /absolute/ceremony/1.A.json /absolute/ceremony/B/root.pem SERIAL_B \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.root.json
bun run @platform/release/src/key/ceremony.ts verify /absolute/ceremony/1.root.json

bun run @platform/release/src/bitwarden/archive.ts /absolute/signing/production /Volumes/USB/destack-signing.age
age -d -i /absolute/recovery-identity.txt /absolute/destack-signing.age | tar -xz -C /absolute/restore
```

## Custody

Each key operation and each lost or compromised key has one requirement.

- Sign: verify fingerprints, roles, threshold and expiry before touching each key.
- Enroll: verify the physical serial, and keep the Bitwarden credentials after an interruption.
- Rotate or renew root: pass the previous verified root to prepare, sign and verify, and meet both thresholds.
- Back up: two encrypted USB copies, an independent offline recovery identity, and every numbered root.
- Lose one root key: the two remaining keys authorize its replacement.
- Lose two root keys: a separately authenticated reinstall.
- Compromised CI: stop publication, revoke credentials, rotate online keys in a root ceremony, and review releases.

## Provenance

The publish job attests every published archive and installer with a GitHub build provenance attestation.

```sh
gh attestation verify {sha256}.aarch64-apple-darwin.tar.gz --repo destack-sh/destack \
    --signer-workflow destack-sh/destack/.github/workflows/release.yml
gh attestation verify {sha256}.universal-apple-darwin.dmg --repo destack-sh/destack \
    --signer-workflow destack-sh/destack/.github/workflows/release.yml
```

## Apple setup

A [Developer ID Application certificate](https://developer.apple.com/help/account/certificates/create-developer-id-certificates) signs the application, and an App Store Connect key [notarizes](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) it.

```sh
xcrun notarytool submit Destack.dmg --key "$APPLE_NOTARY_KEY" --key-id "$APPLE_NOTARY_KEY_ID" --issuer "$APPLE_NOTARY_ISSUER" --wait
```

## Tests

The tests sign, publish, renew and serve repositories with disposable keys and local buckets, and build and install the executables in a sandbox.

```sh
just @platform/release/test
```

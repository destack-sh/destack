# Distribution runbook

Ship Destack: build, sign and publish the desktop, the CLI and the machine as one distribution, with the first-party releases it bundles.

## Recipes

The `just` recipes check, test, build and pack a distribution, and `release` releases main now.

```sh
just check-package @platform/distribution
just test @platform/distribution
just build
just pack
bun run --cwd @platform/distribution installer
bun run --cwd @platform/distribution release
```

## Commands

`src/main.ts` runs every distribution command as `<noun> <verb> [arguments]`, which the recipes and the release workflow call.

```sh
bun run src/main.ts release prepare              # release: prepare, record
bun run src/main.ts distribution build <target>  # distribution: build, pack, icon, develop, open, install-check
bun run src/main.ts installer build <name>       # installer: build
bun run src/main.ts signing sign sign <target>   # signing: sign, credential
bun run src/main.ts update expiry                # update: sign, publish, renew, verify, expiry
bun run src/main.ts key ceremony verify <root>   # key: ceremony, enroll, generate, rehearse, attest, backup
bun run src/main.ts github credential            # github: credential
```

## Publication

`release.yml` releases main once a day when it moved and its checks pass, or now on `release`, as the month's next `vYYYY.M.N`; it tags the release, records it as a GitHub Release listing the commits since the previous tag, and deploys production with `@platform/deployment`.

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
    record:
        needs: [prepare, publish]
    deploy:
        needs: [prepare, publish]
    renew:
        needs: [prepare]
    check-expiry:
        needs: [prepare, publish, renew]
    verify:
        needs: [prepare, publish]
```

## Renewal

The daily renewal publishes the newly signed timestamp and snapshot, replacing the timestamp at the revision it read, as a release does.

```sh
bun run src/main.ts update publish dist/refresh
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

## Builds

`Profile` gives a build its identity: `release` for published releases and `local` for a checkout's builds, each with its own application title, identifier, command, icons and device directory.
A release takes published updates and signs in at `https://destack.app`, and a local build signs in at `http://destack.localhost:4100`, unless `DESTACK_UNIVERSE_ORIGIN` names another universe.

```ts
import { Profile } from "@destack/update/release";

new Profile("release").title; // "Destack"
new Profile("local").command; // "destack-local"
```

## Downloads

Downloads are the release bucket's objects served at `download.destack.sh` by an R2 custom domain, each with the content type and cache policy its upload stored, and `PUBLIC_PATH` admits only these paths.
The `install` script installs the Linux archive, and the first launch of the macOS DMG's application registers the CLI and the machine.

```sh
curl -fsSL https://download.destack.sh/install | sh                 # the bootstrap script
curl https://download.destack.sh/downloads.json                     # the bootstrap index
curl https://download.destack.sh/metadata/timestamp.json            # the feed's TUF metadata
curl -O https://download.destack.sh/targets/<sha256>.<target>.tar.gz  # signed manifests, archives, installers
curl -O https://download.destack.sh/targets/blobs/<sha256>           # every release file once, gzip
```

## Keys

The root role signs with two of three YubiKeys and a physical touch, and `readReleaseKeys` reads one environment's Ed25519 targets, snapshot and timestamp keys from `DESTACK_RELEASE_KEYS`.

```ts
const { targets, snapshot, timestamp } = await readReleaseKeys();
```

### Expiry

`update expiry` fails once root is within 90 days of expiring, targets within 30, and snapshot and timestamp within 3.

```sh
bun run @platform/distribution/src/main.ts update expiry
```

## Environments

Each GitHub environment holds only the secrets and variables its jobs sign and upload with; `@platform/deployment` lists `production`'s.

```yaml
release:
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
renewal:
    secrets:
        [
            DESTACK_SNAPSHOT_KEY,
            DESTACK_TIMESTAMP_KEY,
            CLOUDFLARE_RELEASE_ACCESS_KEY_ID,
            CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY,
        ]
build:
    secrets: []
```

## Ceremony

`key enroll` enrolls a YubiKey, and `key ceremony` prepares a root, signs it with two YubiKeys and verifies it.

```sh
brew install ykman opensc yubico-piv-tool age
export BW_SESSION="$(bw unlock --raw)"

bun run @platform/distribution/src/main.ts key enroll SERIAL /absolute/ceremony/A
bun run @platform/distribution/src/main.ts key rehearse /absolute/ceremony/A/root.pem SERIAL \
    /opt/homebrew/lib/libykcs11.dylib
bun run @platform/distribution/src/main.ts key generate /absolute/private/release

bun run @platform/distribution/src/main.ts key ceremony prepare \
    /absolute/private/release \
    /absolute/ceremony/A/root.pem /absolute/ceremony/B/root.pem /absolute/ceremony/C/root.pem \
    EXPIRY /absolute/ceremony/1.unsigned.json
bun run @platform/distribution/src/main.ts key ceremony sign \
    /absolute/ceremony/1.unsigned.json /absolute/ceremony/A/root.pem SERIAL_A \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.A.json
bun run @platform/distribution/src/main.ts key ceremony sign \
    /absolute/ceremony/1.A.json /absolute/ceremony/B/root.pem SERIAL_B \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.root.json
bun run @platform/distribution/src/main.ts key ceremony verify /absolute/ceremony/1.root.json

bun run @platform/distribution/src/main.ts key backup /absolute/signing/production \
    /absolute/USB/destack/backup/release/destack-signing-YYYY-MM-DD.tar.gz.age
set -o pipefail
age -d -i /absolute/release-recovery-identity.txt /absolute/destack-signing-YYYY-MM-DD.tar.gz.age \
    | tar -xz -C /absolute/restore
```

## Custody

Each key operation and each lost or compromised key has one requirement.

- Sign: verify fingerprints, roles, threshold and expiry before touching each key.
- Enroll: verify the physical serial, and keep the Bitwarden credentials after an interruption.
- Rotate or renew root: pass the previous verified root to prepare, sign and verify, and meet both thresholds.
- Back up release signing: retain every numbered root and two encrypted archive copies on E and F; `key backup` stores the archive's recovery identity in Bitwarden.
- Lose one root key: the two remaining keys authorize its replacement.
- Lose two root keys: a separately authenticated reinstall.
- Compromised CI: stop publication, revoke credentials, rotate online keys in a root ceremony, and review releases.

### Backup media

E and F are physical sticks, both possibly labelled `NO NAME` and stored apart, each archive kept under its basename with its macOS `._*` companion and verified before and after a move.

```sh
ls /Volumes/E/destack/backup/release /Volumes/E/destack/backup/apple
# destack-signing-2026-09-24.tar.gz.age   destack-apple-2026-10-01.tar.gz.age
ls /Volumes/F/destack/backup/release /Volumes/F/destack/backup/apple
# destack-signing-2026-09-24.tar.gz.age   destack-apple-2026-10-01.tar.gz.age
```

### Recovery

Each backup recovers through its own record; the Workers' root keys recover through `@platform/deployment`'s custody record.

| Backup           | Recovers through                                    |
| ---------------- | --------------------------------------------------- |
| Release archive  | Bitwarden `Destack / Release / Backup / 2026-09-24` |
| Apple archive    | Bitwarden `Destack / Release / Apple / 2026-10-01`  |
| TUF root signing | the YubiKey threshold                               |

### Records

Release tooling resolves enrolled records through the stable item IDs in `~/.destack-signing/bitwarden.json`, so a rename of their display names keeps that mapping.

| Record                 | Bitwarden item                                                      |
| ---------------------- | ------------------------------------------------------------------- |
| GitHub App credentials | `Destack / GitHub / App`                                            |
| YubiKeys A, B and C    | `Destack / Release / Signing / YubiKey A (23503030)`, B and C alike |
| YubiKey D              | `Destack / Release / Rehearsal / YubiKey D (23503038)`              |

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
xcrun notarytool submit Destack.dmg --wait \
    --key "$APPLE_NOTARY_KEY" --key-id "$APPLE_NOTARY_KEY_ID" --issuer "$APPLE_NOTARY_ISSUER"
```

## Tests

The tests sign, publish, renew and serve repositories with disposable keys and local buckets, and build and install the executables in a sandbox.

```sh
just test @platform/distribution
```

# @platform/release

Ship Destack: build, sign and publish the desktop, the CLI and the machine as one distribution, and deploy the operated platform (its universe, cloud machines and OpenTofu roots).

## Recipes

The `just` recipes check, build and pack a distribution, and `release` releases main now.

```sh
just @platform/release/check
just @platform/release/lint
just @platform/release/format-check
just @platform/release/test
just @platform/release/build
just @platform/release/pack
just @platform/release/installer
just @platform/release/release
```

## Commands

`src/main.ts` runs every release command as `<noun> <verb> [arguments]`, which the recipes and the release workflow call.

```text
release       prepare | record                           src/release
distribution  build | pack | icon | develop | open | install-check   src/distribution
installer     build                                      src/installer
signing       sign | credential                          src/signing
update        sign | publish | renew | verify | expiry   src/repository
key           ceremony | enroll | generate | rehearse | attest | backup   src/key
platform      validate | format | format-check | init | diff | deploy | rehearse | e2e   src/deployment
custody       seal                                       src/deployment
local         serve | reset                              src/local
github        credential                                 src/github
```

## OpenTofu roots

The OpenTofu roots and their modules live under `src/deployment`, and `deployment/*.tfvars` configures each root per stage.

```text
src/deployment/roots     global | universe | residency
src/deployment/modules   kms | database
deployment/*.tfvars      each root's configuration per stage
```

## Deployments

`Deployment` is the platform at a stage and declares its universe: the residencies, the platform with its reserved handles, and one cloud machine per residency at its origin under the key its root keys derive.

```ts
const local = Deployment.of("local"); // issuer http://127.0.0.1:4100, the local stage's
const declaration = await local.universe(
    "operator@destack.test",
    (residency) => LocalKeyring.read(rootKey, host),
    await build.stacks(local.settings), // the homes' stacks at the checkout's platform build
    host,
);
// https://eu-central--platform.destack.computer
Stage.deployed("production").machineOrigin(RESIDENCIES[0]);
```

## Local universe

`just local` migrates `destack_dev` on the PostgreSQL of `just postgres up`, starts the universe, a cloud machine per residency and the local stage's machine, and stops them on Ctrl-C.

```sh
# universe 127.0.0.1:4100, eu-central :4102, us-east :4103, machine :4101; mail printed
just local
# drop destack_dev, target/dev and the local stage's machine with its keychain entries
just local reset
# serve or drop another state: destack_slice, target/slice, its machine in target/slice/device
just local serve --state slice
just local reset --state slice
# in @destack/cli, with DESTACK_DIRECTORY=target/<state>/device for another state
DESTACK_STAGE=local bun run src/main.ts login \
    --email ada@destack.test --handle ada --residency eu
```

## Workers

`WorkerScript` deploys the entries: the universe's `@platform/release/deployment/universe`, and per residency `@destack/machine/cloudflare/worker` with its Tail Worker `@destack/machine/cloudflare/tail`, which posts each space's traced CPU time and Durable Object fetches to the space's object.

```sh
# bundle every script and build its upload with wrangler --dry-run
just @platform/release/rehearse production
# serve the local universe and cloud machines under workerd over a fresh database, smoke
just @platform/release/e2e
# seal the keys custody/production.json lacks under each root's KMS key, once per key, then commit the record
just @platform/release/custody seal production
# apply global, universe, eu and us, migrate, deploy each Worker with the shipped releases as its
#  static assets, the record's sealed keys and its KMS access key, then publish the releases and smoke
just @platform/release/deploy production
```

### Bundles

Each bundle carries the data its deploy supplies as `destack:` modules, since a Worker variable holds at most 5 KB.

```ts
import distribution from "destack:distribution"; // the distribution's manifest
import universe from "destack:universe"; // the universe's declaration
```

## Key custody

Each Worker's root keys are sealed by an AWS KMS key, as Vault's auto-unseal and the Kubernetes KMS provider seal theirs.

```text
custody/production.json   universe { seal, rootKey }, machines { <id>: { seal, rootKey, publicKey, key } }
  rootKey                  root key versions generated inside KMS (GenerateDataKeyWithoutPlaintext), ciphertexts only
  key                      the machine's P-256 key, generated once by custody seal and encrypted for the machine alone
Worker                     Envelope.keyring(DESTACK_ROOT_KEY) and Envelope.open(DESTACK_MACHINE_KEY): one Decrypt per isolate
```

### Key boundaries

Who may use, change or reach the root keys.

```text
The universe root and each residency root declare their key, its replica and the Worker's IAM user, which may only decrypt under its own `owner` context.
The record holds ciphertexts and public keys only, so it is committed.
No root key exists in the clear at rest: root versions are generated inside KMS, and the account may generate and encrypt under the keys, never decrypt.
Only the Worker's IAM user decrypts, under its own `owner` context; every unwrap lands in CloudTrail.
The key policy names no source addresses: Cloudflare publishes none for Workers' outgoing requests.
The operator still deploys the Worker's access key, so a release run can unseal through it: alert on any `Decrypt` by a `*-kms` user outside a Worker's start.
A machine's key is generated once by `custody seal` and encrypted at once.
Deploys only read the record, refusing it when it lacks a key or keeps one under another key than its root's.
`custody seal` generates only missing keys and refuses a key sealed under another KMS key: rewrap it, never regenerate it, as regenerating orphans everything it sealed.
The operator's Identity Center role (`kms_operators`) creates the root keys and their Workers' IAM users, and alone may repolicy, regrant or replicate a key.
No IAM principal may disable or schedule the deletion of a root key: the key policy denies it to all but the operator, and the operator's permission set leaves it out, so only the account's root user can.
The deploy role creates no key, tag or key policy and seals nothing: a deploy that would change a key fails until the operator applies it and seals the record.
The operator can rewrite a key's policy and so grant itself `Decrypt`, behind Identity Center's sign-in MFA and CloudTrail.
The deploy pipeline still reaches plaintext indirectly: the state it reads holds the Workers' access keys, and the Worker code it deploys decrypts; its limits are least privilege, and the GitHub `production` environment's required reviewer is the boundary against a compromised pipeline.
Root keys carry a `destack-root` tag from their creation; the Workers' IAM users sit under the `destack-kms-decrypt-boundary` permissions boundary.
Losing the record loses the roots; the keys keep a replica in a second region of the same jurisdiction.
```

## Roots

Each root keeps its own remote state, which the operator plans and applies through the release commands.

```sh
bun run src/main.ts platform validate                # validate every root offline
just @platform/release/diff production universe      # plan
just @platform/release/deploy production us          # plan and apply one root
```

### State encryption

How each root's state stays encrypted.

```text
Each root's state and plans are encrypted under the KMS key `alias/destack-state` in `eu-central-1` (OpenTofu's `aes_gcm` over `aws_kms`), which only the deploy role and the operator may use.
A state written before encryption moves under the key once, by the operator: add `method "unencrypted" "migrate" {}` and `fallback { method = method.unencrypted.migrate }` beside `state` in the root's `encryption.tf`, apply, then remove both; the next plan reads the state encrypted.
The universe and residency roots are applied first by the operator, which creates their keys; the deploy role applies them afterwards.
```

## Publication

`release.yml` releases main once a day when it moved and its checks pass, or now on `release`, as the month's next `vYYYY.M.N`; it tags the release, records it as a GitHub Release listing the commits since the previous tag, and deploys production once the repository variable `DESTACK_PLATFORM` is `deployed`.

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

## Downloads

Downloads are the release bucket's objects served at `download.destack.sh` by an R2 custom domain, each with the content type and cache policy its upload stored.

```text
download.destack.sh/install          the bootstrap script
download.destack.sh/downloads.json   the bootstrap index
download.destack.sh/metadata/…       the feed's TUF metadata
download.destack.sh/targets/…        the feed's content-addressed archives and installers
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

## Download layout

`PUBLIC_PATH` admits the TUF metadata, content-addressed targets, `downloads.json` and `install` the feed publishes at the root of `download.destack.sh` from its R2 bucket.

```sh
curl https://download.destack.sh/metadata/timestamp.json
curl https://download.destack.sh/downloads.json
```

## Builds

`Stage` gives a build its identity: `production` for published releases, `dev` for local builds, each with its own application title, identifier, command, icons and device directory.

```ts
import { Stage } from "@destack/update/release";

Stage.of("production").title; // "Destack"
Stage.of("local").command; // "destack-local"
```

## Downloads

The `install` script installs the Linux archive, and the first launch of the macOS DMG's application registers the CLI and the machine.

```sh
curl -fsSL https://download.destack.sh/install | sh
```

## Keys

The root role signs with two of three YubiKeys and a physical touch, and `readReleaseKeys` reads one environment's Ed25519 targets, snapshot and timestamp keys from `DESTACK_RELEASE_KEYS`.

```ts
const { targets, snapshot, timestamp } = await readReleaseKeys();
```

### Expiry

`update expiry` fails once root is within 90 days of expiring, targets within 30, and snapshot and timestamp within 3.

```sh
DESTACK_STAGE=production bun run @platform/release/src/main.ts update expiry
```

## Environments

Each GitHub environment holds only the secrets and variables its jobs sign, upload and deploy with.

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
production:
    secrets: [CLOUDFLARE_COMPANY_API_TOKEN, PLANETSCALE_SERVICE_TOKEN_ID, PLANETSCALE_SERVICE_TOKEN]
    variables: [DESTACK_OPERATOR]
build:
    secrets: []
```

## Ceremony

`key enroll` enrolls a YubiKey, and `key ceremony` prepares a root, signs it with two YubiKeys and verifies it.

```sh
brew install ykman opensc yubico-piv-tool age
export BW_SESSION="$(bw unlock --raw)"

bun run @platform/release/src/main.ts key enroll SERIAL /absolute/ceremony/A
bun run @platform/release/src/main.ts key rehearse /absolute/ceremony/A/root.pem SERIAL \
    /opt/homebrew/lib/libykcs11.dylib
bun run @platform/release/src/main.ts key generate /absolute/private/release

bun run @platform/release/src/main.ts key ceremony prepare \
    /absolute/private/release \
    /absolute/ceremony/A/root.pem /absolute/ceremony/B/root.pem /absolute/ceremony/C/root.pem \
    EXPIRY /absolute/ceremony/1.unsigned.json
bun run @platform/release/src/main.ts key ceremony sign \
    /absolute/ceremony/1.unsigned.json /absolute/ceremony/A/root.pem SERIAL_A \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.A.json
bun run @platform/release/src/main.ts key ceremony sign \
    /absolute/ceremony/1.A.json /absolute/ceremony/B/root.pem SERIAL_B \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.root.json
bun run @platform/release/src/main.ts key ceremony verify /absolute/ceremony/1.root.json

bun run @platform/release/src/main.ts key backup /absolute/signing/production \
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

```text
E/destack/backup/
  release/destack-signing-2026-09-24.tar.gz.age
  apple/destack-apple-2026-10-01.tar.gz.age

F/destack/
  backup/release/destack-signing-2026-09-24.tar.gz.age
  backup/apple/destack-apple-2026-10-01.tar.gz.age
```

### Recovery

Each backup recovers through its own record.

```text
release archive              Bitwarden `Destack / Release / Backup / 2026-09-24`
Apple archive                Bitwarden `Destack / Release / Apple / 2026-10-01`
Worker root keys             KMS, through `custody/<stage>.json`
TUF root signing             the YubiKey threshold
```

### Records

Release tooling resolves enrolled records through the stable item IDs in `~/.destack-signing/bitwarden.json`, so a rename of their display names keeps that mapping.

```text
GitHub App credentials       Bitwarden `Destack / GitHub / App`
YubiKeys A, B and C          Bitwarden `Destack / Release / Signing / YubiKey A (23503030)`, B and C alike
YubiKey D                    Bitwarden `Destack / Release / Rehearsal / YubiKey D (23503038)`
```

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
just @platform/release/test
```

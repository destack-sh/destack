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

`release-publish` builds and publishes one channel per run: nightly from main, stable on a pushed `v<version>` tag, either on a manual dispatch.

```text
release-publish   a nightly at the published nightly's commit skips build and only renews
├── prepare     select version and work                    github/prepare.ts
├── check       workspace-check
├── build       build, verify, pack, rehearse per target   build.ts, verify.ts, pack.ts, rehearse.ts
├── sign        sign and notarize per signed target        sign.ts, apple/credential.ts
├── installer   build per installer                        installer/build.ts
├── publish     attest archives and installers, sign       github/credential.ts, sign.ts, upload.ts, verify.ts
│               targets, snapshot and timestamp
├── renew       re-sign every other channel's snapshot     github/credential.ts, renew.ts, submit.ts, verify.ts
│               and timestamp
├── expiry      fail inside a signed role's renewal window expiry.ts
└── verify      install and update per target              install.ts
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
    bundle: APPLE_BUNDLE,      // Destack.app, Contents/MacOS/Destack, Contents/Helpers
    signer: "apple",           // MacSigning: sign, notarize, verify; Linux rows name none
    installers: [UNIVERSAL_DMG],
}
```

## Download layout

Each channel publishes its TUF metadata, targets and installer below `download.destack.sh`, and stores them in a separate R2 bucket.

```text
download.destack.sh/{stable,nightly}/
├── metadata/{version}.{root,targets,snapshot}.json
├── metadata/timestamp.json
├── targets/{sha256}.{target}.{tar.gz,dmg}
├── downloads.json
└── install

destack-release-publication → destack-releases-{stable,nightly} (R2)
```

## Channels

A stable release waits for approval and a nightly release publishes unattended, and the channels number their versions differently.

```text
stable        tag v<version> on main          2026.10.1                         approval required
nightly       daily or dispatched from main   2026.10.{run}-nightly.{attempt}   unattended
development   local build                     the Desktop package version       unpublished

each channel  its own application title, identifier, command and icons, updating only from its own repository
```

## Downloads

The `install` shell script installs the Linux archive, and the first launch of the macOS application registers the CLI and the daemon.

```text
macOS DMG        the universal application
Linux archive    the distribution for the install script
update archive   the complete distribution for one OS and CPU
```

## Keys

The root role signs with two of three YubiKeys and a physical touch, and the targets, snapshot and timestamp roles each sign with an Ed25519 key per environment.

```text
root        A/B/C YubiKeys, RSA-2048 PIV slot 9c              one year, renewed 90 days early
targets     Ed25519 key per release environment               one year, renewed 30 days early
snapshot    Ed25519 key per release and renewal environment   fourteen days, re-signed by every run, 3 days early
timestamp   Ed25519 key per release and renewal environment   fourteen days, re-signed by every run, 3 days early
```

## Environments

Each GitHub environment holds only the keys and credentials its jobs sign and upload with.

```text
release-{stable,nightly}   DESTACK_TARGETS_KEY, DESTACK_SNAPSHOT_KEY, DESTACK_TIMESTAMP_KEY,
                           CLOUDFLARE_RELEASE_ACCESS_KEY_ID, CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY,
                           APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_NOTARY_KEY
renewal-{stable,nightly}   DESTACK_SNAPSHOT_KEY, DESTACK_TIMESTAMP_KEY
build                      no secrets
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

```text
sign                   verify fingerprints, roles, threshold and expiry before touching each key
enroll                 verify the physical serial, and keep the Bitwarden credentials after an interruption
rotate or renew root   pass the previous verified root to prepare, sign and verify, and meet both thresholds
back up                two encrypted USB copies, an independent offline recovery identity, every numbered root
lose one root key      the two remaining keys authorize its replacement
lose two root keys     a separately authenticated reinstall
compromised CI         stop publication, revoke credentials, rotate online keys in a root ceremony, review releases
```

## Provenance

The publish job attests every published archive and installer with a GitHub build provenance attestation.

```sh
gh attestation verify {sha256}.aarch64-apple-darwin.tar.gz --repo destack-sh/destack \
    --signer-workflow destack-sh/destack/.github/workflows/release-publish.yml
gh attestation verify {sha256}.universal-apple-darwin.dmg --repo destack-sh/destack \
    --signer-workflow destack-sh/destack/.github/workflows/release-publish.yml
```

## Platform signing

`release-publish.yml` passes these Apple secrets and variables to signing and notarization.

```text
secrets     APPLE_CERTIFICATE (base64 P12), APPLE_CERTIFICATE_PASSWORD, APPLE_NOTARY_KEY (P8)
variables   APPLE_SIGNING_IDENTITY, APPLE_NOTARY_KEY_ID, APPLE_NOTARY_ISSUER
```

## Apple setup

A Developer ID Application certificate signs the application, and an App Store Connect key notarizes it.

```text
certificate    https://developer.apple.com/help/account/certificates/create-developer-id-certificates
notarization   https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
```

## Tests

The tests sign, publish, renew and serve repositories with disposable keys and local buckets, and build and install the executables in a sandbox.

```sh
just @platform/release/test
```

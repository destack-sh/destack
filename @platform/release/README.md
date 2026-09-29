Build, sign and publish Desktop, CLI and daemon as one distribution.

```sh
just platform/release/check
just platform/release/lint
just platform/release/format-check
just platform/release/test
just platform/release/build
just platform/release/pack
just platform/release/installer
just platform/release/rehearse VERSION ARCHIVE
```

## Publication

```text
workspace-check → release-publish → build → sign → pack → publish → verify
release-renew   → sign snapshot + timestamp → POST /{channel}/renew
release-check  → check metadata expiry

download.destack.sh/{stable,nightly}/
├── metadata/{version}.{root,targets,snapshot}.json
├── metadata/timestamp.json
├── targets/{sha256}.{target}.{tar.gz,dmg,exe}
├── downloads.json
├── install
└── install.ps1

destack-release-publication → destack-releases-{stable,nightly} (R2)
```

| Release | Trigger | Version | Approval |
| --- | --- | --- | --- |
| Stable | `v<version>` on main | `2026.9.1` | Required |
| Nightly | Daily, or manual publication from main | `2026.9.{run}-nightly.{attempt}` | Unattended |
| Development | Local build | Desktop's package version | Unpublished |

| Download | Contents |
| --- | --- |
| macOS DMG | Universal application; first launch registers CLI and daemon |
| Windows Setup | Per-user installation and uninstaller |
| Linux archive | Distribution installed by the shell script |
| Update archive | Complete distribution for one OS/CPU |

## Keys

| Role | Custody | Expiry |
| --- | --- | --- |
| Root | A/B/C YubiKeys, RSA-2048 PIV 9c, two signatures and physical touch | One year; renew 90 days early |
| Targets | Separate Ed25519 key per release environment | One year |
| Snapshot | Separate Ed25519 key per release/renewal environment | Seven days |
| Timestamp | Separate Ed25519 key per release/renewal environment | Two days |

| GitHub environment | Secrets |
| --- | --- |
| `release-{stable,nightly}` | `DESTACK_TARGETS_KEY`, `DESTACK_SNAPSHOT_KEY`, `DESTACK_TIMESTAMP_KEY`, `CLOUDFLARE_RELEASE_ACCESS_KEY_ID`, `CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY` |
| `renewal-{stable,nightly}` | `DESTACK_SNAPSHOT_KEY`, `DESTACK_TIMESTAMP_KEY` |
| `build` | None |

## Ceremony

```sh
brew install ykman opensc yubico-piv-tool age
export BW_SESSION="$(bw unlock --raw)"

bun run platform/release/src/key/enroll.ts SERIAL /absolute/ceremony/A
bun run platform/release/src/key/rehearse.ts /absolute/ceremony/A/root.pem SERIAL /opt/homebrew/lib/libykcs11.dylib
bun run platform/release/src/key/generate.ts /absolute/private/stable

bun run platform/release/src/key/ceremony.ts prepare \
    /absolute/private/stable \
    /absolute/ceremony/A/root.pem /absolute/ceremony/B/root.pem /absolute/ceremony/C/root.pem \
    EXPIRY /absolute/ceremony/1.unsigned.json
bun run platform/release/src/key/ceremony.ts sign \
    /absolute/ceremony/1.unsigned.json /absolute/ceremony/A/root.pem SERIAL_A \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.A.json
bun run platform/release/src/key/ceremony.ts sign \
    /absolute/ceremony/1.A.json /absolute/ceremony/B/root.pem SERIAL_B \
    /opt/homebrew/lib/libykcs11.dylib /absolute/ceremony/1.root.json
bun run platform/release/src/key/ceremony.ts verify /absolute/ceremony/1.root.json

bun run platform/release/src/key/archive.ts /absolute/signing/production /Volumes/USB/destack-signing.age
age -d -i /absolute/recovery-identity.txt /absolute/destack-signing.age | tar -xz -C /absolute/restore
```

| Operation | Requirement |
| --- | --- |
| Sign | Verify fingerprints, roles, threshold and expiry before touching each key |
| Enroll | Verify physical serial; retain Bitwarden credentials after interruptions |
| Rotate/renew root | Append previous verified root to `prepare`, `sign`, `verify`; satisfy old and new thresholds |
| Back up | Two encrypted USB copies; independent offline recovery identity; retain every numbered root |
| Lose one root key | Two surviving keys authorize its replacement |
| Lose two root keys | Separately authenticated reinstall; automatic trust recovery is impossible |
| Compromise CI | Stop publication, revoke credentials, rotate online keys through root ceremony, review releases |

## Platform signing

| Platform | GitHub secrets | GitHub variables |
| --- | --- | --- |
| Apple | `APPLE_CERTIFICATE` (base64 P12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_NOTARY_KEY` (P8) | `APPLE_SIGNING_IDENTITY`, `APPLE_NOTARY_KEY_ID`, `APPLE_NOTARY_ISSUER` |
| Windows | None; GitHub OIDC | `AZURE_SIGNING_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`, `AZURE_SIGNING_PROFILE` |

| Setup | Reference |
| --- | --- |
| Apple Developer ID Application certificate | [Create certificate](https://developer.apple.com/help/account/certificates/create-developer-id-certificates) |
| Apple App Store Connect key and notarization | [Notarize](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) |
| Azure Public Trust profile; Certificate Profile Signer scoped to that profile | [Configure Artifact Signing](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart) |

```text
OIDC issuer:   https://token.actions.githubusercontent.com
OIDC audience: api://AzureADTokenExchange
OIDC subjects:
  repo:destack-sh/destack:environment:release-stable
  repo:destack-sh/destack:environment:release-nightly
```

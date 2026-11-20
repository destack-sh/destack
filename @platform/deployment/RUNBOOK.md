# Deployment runbook

Deploy the platform Destack operates: its universe and cloud machines on Cloudflare over OpenTofu roots, or the local universe on this computer.

## Recipes

The `just` recipes check and test the package, and run the local universe.

```sh
just check-package @platform/deployment
just test @platform/deployment
just local serve
just local reset
```

## Commands

`src/main.ts` runs every deployment command as `<noun> <verb> [arguments]`, which the recipes and the release workflow call.

```sh
bun run src/main.ts platform validate            # platform: validate, format, format-check, init, diff, build, deploy, publish
bun run src/main.ts custody seal production      # custody: seal
bun run src/main.ts site deploy                  # site: deploy
bun run src/main.ts local serve                  # local: serve, reset, desktop, cli
```

## Deployments

`Deployment` is the platform deployed for one universe: its address, its tag, each residency's machine and relay origin, and where its mail goes.
The address is the universe's origin with the domains of its spaces, machines and site, and each home's stack gets the routes of its hostnames from it.
`Deployment.cloudflare` reads production, and `Deployment.local` reads a local universe on its loopback ports.

```ts
const production = Deployment.cloudflare("production", host); // https://destack.app, tag latest
const local = Deployment.local(LocalPorts.at(4100), host); // http://destack.localhost:4100, tag local
local.universe.domains; // { space: "space.localhost", machine: "computer.localhost", site: "site.localhost" }
production.machineOrigin(RESIDENCIES[0]); // https://eu-central--platform.destack.computer
```

`Cloudflare` runs a Cloudflare deployment: `deploy` applies the roots and releases the Workers, `release` deploys the Workers alone, and `seal` seals the keys the custody record lacks.
`platform publish` then publishes the checkout's releases into the platform accounts' forges through the universe, signed in as the workflow's trusted publisher.

## Local universe

`just local` creates `destack_local` on the PostgreSQL of `just postgres up`, starts the universe, a cloud machine per residency and the local universe's machine, and stops them on Ctrl-C.
Each local universe keeps its database, its directory under `target/` and its own block of ports, so several run side by side.

```sh
# universe destack.localhost:4100 and the names below space.localhost and computer.localhost served by
#  eu-central's machine, this computer's machine :4101, us-east :4102; mail printed
just local serve
# the same universe on workerd, as production runs it
just local serve --runtime workerd
# drop the universe's database, its directory and its machine with its keychain entries
just local reset
# serve or drop another local universe: destack_slice, target/slice, the next free ports (4110, 4120, ...)
just local serve --universe slice
just local reset --universe slice
# run the cli or the desktop app on a local universe
just local cli --universe slice login --email ada@destack.test --handle ada --residency eu
```

## Workers

`WorkerScript` deploys the entries: the universe's `@destack/universe/cloudflare/worker`, and per residency `@destack/machine/cloudflare/worker` with its Tail Worker `@destack/machine/cloudflare/tail`, which posts each space's traced CPU time and Durable Object fetches to the space's object.
The universe's Worker and `BunUniverse` are `@destack/universe`'s own hosts; each opens the distribution's releases from its layer, and every other release at its publisher's forge.
The platform residency's machine serves the universe's domain: it hands the universe's paths to the universe, through its binding on Workers and in process on Bun, and serves the rest from the routes of the platform's home, Home's start among them.

```sh
# serve the local universe and cloud machines as Workers, as production runs them
just local serve --runtime workerd
# seal the keys custody/production.json lacks under each root's KMS key, once per key, then commit the record
bun run --cwd @platform/deployment custody seal production
# apply global, universe, eu and us, and deploy each Worker with the checkout's releases as its
#  static assets, the record's sealed keys and its KMS access key
bun run --cwd @platform/deployment deploy production
```

### Bundles

Each bundle carries the data its deploy supplies as `destack:` modules, since a Worker variable holds at most 5 KB.

```ts
import distribution from "destack:distribution"; // the distribution's manifest
import universe from "destack:universe"; // the universe's declaration
```

## Key custody

Each Worker's root keys are sealed by an AWS KMS key, as Vault's auto-unseal and the Kubernetes KMS provider seal theirs.
The record `custody/<universe>.json` holds ciphertexts and public keys only: root key versions generated inside KMS (`GenerateDataKeyWithoutPlaintext`), and each machine's P-256 key, generated once by `custody seal` and encrypted for that machine.

```json
{
    "universe": { "seal": "alias/destack-universe", "rootKey": ["<ciphertext>"] },
    "machines": {
        "<machine id>": {
            "seal": "alias/destack-eu-central",
            "rootKey": ["<ciphertext>"],
            "publicKey": "<P-256 public key>",
            "key": "<ciphertext>"
        }
    }
}
```

A Worker unseals both with one `Decrypt` each per isolate.

```ts
const keyring = await Envelope.keyring(env.DESTACK_ROOT_KEY);
const machineKey = await Envelope.open(env.DESTACK_MACHINE_KEY);
```

### Key boundaries

Who may use, change or reach the root keys.

- **Keys:** the universe root and each residency root declare their key, its replica in a second region of the same jurisdiction, and the Worker's IAM user.
- **At rest:** no root key exists in the clear; the account may generate and encrypt under the keys, never decrypt, so the record is committed.
- **Decrypt:** only a Worker's IAM user decrypts, under its own `owner` context, and every unwrap lands in CloudTrail.
- **Network:** the key policy names no source addresses, as Cloudflare publishes none for Workers' outgoing requests.
- **Access keys:** the operator deploys each Worker's access key, so alert on any `Decrypt` by a `*-kms` user outside a Worker's start.
- **Sealing:** `custody seal` generates only missing keys and refuses a key sealed under another KMS key; rewrap it, never regenerate it, as regenerating orphans everything it sealed.
- **Deploys:** they only read the record, and refuse it when it lacks a key or keeps one under another key than its root's.
- **Operator:** the Identity Center role `kms_operators` creates the root keys and their Workers' IAM users, and alone may repolicy, regrant or replicate a key.
- **Deletion:** no IAM principal may disable or schedule the deletion of a root key, so only the account's root user can.
- **Deploy role:** it creates no key, tag or key policy and seals nothing, so a deploy that would change a key fails until the operator applies it.
- **Policy rewrite:** the operator can grant itself `Decrypt` by rewriting a key's policy, behind Identity Center's sign-in MFA and CloudTrail.
- **Pipeline:** the state it reads holds the Workers' access keys and the code it deploys decrypts, so the GitHub `production` environment's required reviewer is the boundary against a compromised pipeline.
- **Tags:** root keys carry `destack-root` from their creation, and the Workers' IAM users sit under the `destack-kms-decrypt-boundary` permissions boundary.
- **Loss:** losing the record loses the roots.

## Roots

The roots `access`, `global`, `universe` and `residency` live under `src/deployment/roots` with the modules `database`, `kms` and `worker`, and `deployment/<universe>.tfvars` configures each per universe.
Each root keeps its own remote state, which the operator plans and applies through the deployment commands.

```sh
bun run src/main.ts platform validate                 # validate every root offline
bun run src/main.ts platform diff production universe # plan
bun run src/main.ts platform deploy production us     # plan and apply one root
```

### State encryption

Each root's state and plans are encrypted under the KMS key `alias/destack-state` in `eu-central-1` with OpenTofu's `aes_gcm` over `aws_kms`, which only the deploy role and the operator may use.
The operator applies the universe and residency roots first, which creates their keys, and the deploy role applies them afterwards.
Only an operator applies the access root, which holds every platform identity, the workflows' roles and the people's permission sets.
A state written before encryption moves under the key once: the operator adds the fallback beside `state` in the root's `encryption.tf`, applies, then removes both blocks.

```hcl
method "unencrypted" "migrate" {}

state {
    method = method.aes_gcm.state
    fallback {
        method = method.unencrypted.migrate
    }
}
```

## Environment

The GitHub `production` environment holds only the secrets and variables the deploy and publish jobs use.

```yaml
production:
    secrets:
        [
            CLOUDFLARE_COMPANY_API_TOKEN,
            GH_CLIENT_SECRET,
            GOOGLE_CLIENT_SECRET,
            MICROSOFT_CLIENT_SECRET,
        ]
    variables: [DESTACK_OPERATOR]
```

## Tests

The tests declare both deployments' universes, select the roots and their state, and seal and open custody records with disposable keys.

```sh
just test @platform/deployment
```

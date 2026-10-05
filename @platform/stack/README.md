# @platform/stack

Destack's operated platform stack: the OpenTofu roots and the dev universe.

## Placement

`Placement` maps the platform's workloads to processes by the cardinality each declares: `PLATFORM` lists account, forge and relay with the database each serves over.

```text
workload     placement        cloud Worker                         dev universe
account      once             destack-<env>-account                universe
forge        per-residency    destack-<env>-forge-eu, …-us         universe
relay        once             destack-<env>-relay                  universe, its own listener
cell         per-zone, host   placed by cells and daemons
```

```ts
// RESIDENCIES in src/universe/residency.ts declares each residency's code, regions and Durable Object location
Placement.spread(PLATFORM).processes; // [{ name: "account", packages }, { name: "forge-eu", residency: { code: "eu", … }, packages }, …]
Placement.single("universe", PLATFORM).processes; // one process with every workload
```

## Workers

Each cloud process is a Worker whose one Durable Object runs its workload, and `WorkerProcess.forward` hands every request to the object.

```text
# the object runs in its residency's jurisdiction near its database, and its alarm wakes the workload's controllers
# it opens PostgreSQL through Hyperdrive as the sole writer, so commits wake readers in memory without LISTEN
process        entry                               origin and routes                         bindings
account        src/cloudflare/worker/account.ts    destack.app (custom domain)               DATABASE
forge-eu       src/cloudflare/worker/forge.ts      eu.destack.cloud/service/<forge>*         DATABASE, PACKAGES (R2, eu), ACCOUNT
relay          src/cloudflare/worker/relay.ts      relay.destack.space, *.destack.space/*,   DATABASE, ACCOUNT
                                                   *.destack.computer/*
```

```sh
# bundle every Worker with its module metadata and build it with wrangler deploy --dry-run over placeholder identifiers
just rehearse production  # .wrangler/production/<process>/{index.js,wrangler.json,dist}
```

```text
# the secrets an operator puts with wrangler secret bulk before a Worker's first deploy
# placed processes run as the placement the operator created, proving the region's host with its enrolled key
every process          DESTACK_CALL_KEY (32 bytes as hexadecimal)
account                DESTACK_SECRET, DESTACK_OPERATOR (the operator's email), AWS_ACCESS_KEY_ID,
                       AWS_SECRET_ACCESS_KEY
forge                  DESTACK_PLACEMENT_ID, DESTACK_HOST_ID, DESTACK_HOST_KEY, DESTACK_REGION_ID,
                       DESTACK_ARTIFACTS_TOKEN
relay                  DESTACK_PLACEMENT_ID, DESTACK_HOST_ID, DESTACK_HOST_KEY
```

## Migrations

A release migrates each process's database before its Worker deploys: `just migrate <process>` creates the schema named after the connecting role, which the role's search path reads first, and migrates it.

```sh
DATABASE_URL=postgres://forge-eu:…@…/postgres just migrate forge-eu
```

## Development universe

`just universe` serves the dev universe at `UNIVERSE_ORIGIN` and its relay at `RELAY_ORIGIN`, every service in its own schema of the local PostgreSQL database `destack_dev`, files in `target/universe`.

```sh
just postgres start
just @platform/stack/universe
export DESTACK_RELAY=http://127.0.0.1:4101/tunnel  # a dev daemon's tunnel to the universe's relay
```

```ts
// each start creates only what is missing, in this order:
// the RESIDENCIES rows (src/universe/residency.ts), kept by the account service
// the platform organisation Destack with a shared account for each of PLATFORM_HANDLES (src/universe/handle.ts),
//  kept by the account service before it serves, which others read as taken
// the operator's owner invitation, which the operator accepts after signing in
// every residency's regions
// a fresh host of the served region, a placement of each residency workload and the relay's role
await DevelopmentUniverse.start({ region: "eu-central", operator: "operator@destack.test", ... });
```

## Mail

`signInMail` sends sign-in links and codes through Amazon SES when all mail variables are set, prints them when `DESTACK_MAIL_REGION` and `DESTACK_MAIL_FROM` are unset, and refuses a partial set.
The production account Worker sends through SES, and the development one prints to its logs, so only production needs the AWS keys.

```sh
export DESTACK_MAIL_REGION=eu-central-1
export DESTACK_MAIL_FROM="Destack <sign-in@destack.app>"
export AWS_ACCESS_KEY_ID=...      # the mail_access_key_id output of the shared root
export AWS_SECRET_ACCESS_KEY=...  # the mail_secret_access_key output of the shared root
just @platform/stack/universe
```

### Delivery

The universe logs the delivery `signInMail` selected at startup.

```text
mail: printing sign-in links and codes here, since DESTACK_MAIL_REGION and DESTACK_MAIL_FROM are unset
```

## Deployment

`just diff` plans a deployment, `just drift` refuses one whose infrastructure differs from its code, and `just deploy` applies it, asking for confirmation outside CI.

```text
deployment                    root             processes
shared                        src/shared       none: domains, mail, release and site
<environment>/universe        src/universe     the processes serving every residency
<environment>/{eu,us}         src/residency    the processes of one residency
```

```sh
aws sso login --profile destack
AWS_PROFILE=destack just diff shared
just deploy development universe
just deploy development eu
```

## Releases

`just release` migrates each placed process's database as its role and deploys its Worker over the applied deployment's outputs, and checks each origin answers; `just secrets` puts a process's secrets once before its first release, since a Worker keeps them across deploys.

```sh
just deploy development universe
just secrets development account     # DESTACK_CALL_KEY, DESTACK_SECRET, … from the environment
just release development             # migrate account, forge-eu, forge-us, relay; wrangler deploy each; smoke
```

## Workflow

The `Release` workflow deploys `development` after each nightly and `production` after each `v…` tag; `Plan stack` plans pull requests.

```text
pull request    Plan stack   plan shared and the production deployments it touches, in one comment
nightly         Release      check, refuse production drift, publish nightly, apply shared and development/{universe,eu,us}, release development
tag v…          Release      check, publish stable (approval), apply production/{universe,eu,us}, release production
fork            none         no plan
```

## Secrets

The workflows read these secrets in their `plan`, `drift`, `development` and `production` environments, and an operator puts each process's own secrets with `just secrets`.

```text
CLOUDFLARE_COMPANY_API_TOKEN   Cloudflare, wrangler and the R2 state bucket
PLANETSCALE_SERVICE_TOKEN_ID   the PlanetScale service token id
PLANETSCALE_SERVICE_TOKEN      the PlanetScale service token
```

## Tests

The tests check the deployment roots and state keys and drive the dev universe from sign-in to a resolved build.

```sh
just postgres start
just @platform/stack/test
```

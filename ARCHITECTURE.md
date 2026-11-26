# Architecture

Destack is composed of 8 layers, from the desktop and the terminal people open down to the core every package uses.
Each layer is a set of packages under [`@destack`](@destack), and a package depends only on packages in its own layer or below; `destack-check` refuses a dependency on a higher layer.
The tables below list what each package does, in its own description, and the prior art it follows.

## 01 Application

What users interact with: the desktop, the terminal, and any userland apps.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/desktop`](@destack/desktop) | Open Destack apps in desktop windows, with banners, a badge, a tray and a command palette. | Tauri |
| [`@destack/cli`](@destack/cli) | Use Destack from the terminal: sign in, install packages and work with a space's objects. | kubectl, gh |
| [`@destack/template`](@destack/template) | Show an empty page with Destack's mark. | create-vite's vanilla template |

## 02 Runtime

Where packages run: machines, the spaces they serve, and the relays reaching them.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/machine`](@destack/machine) | Run a machine: serve the spaces placed on it with their modules (buckets, vaults, connections, messages, observability, audit), route their installations' calls and keep their builds. | — |
| [`@destack/relay`](@destack/relay) | Relay HTTP requests for Destack names to the machines serving them. | Cloudflare Tunnel, ngrok |
| [`@destack/tunnel`](@destack/tunnel) | Carry HTTP requests over one WebSocket session between a machine and a relay, as a tunnel. | Cloudflare Tunnel |
| [`@destack/universe`](@destack/universe) | Run a universe: the account, object, setting and universe packages over one store at one origin. | Kubernetes control plane |

## 03 Service

What apps build on for shared work: stacks, connections, messages, notifications, billing, repositories and packages.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/stack`](@destack/stack) | Declare and apply a space's stack: its installations, resources, bindings, deployments, routes, runs and transfers. | Pulumi, CloudFormation |
| [`@destack/connection`](@destack/connection) | Connect external accounts once, and call their APIs with credentials kept in a vault. | Nango |
| [`@destack/workflow`](@destack/workflow) | Run durable functions step by step as objects of the space they run in. | Temporal, DBOS, Inngest, Restate |
| [`@destack/message`](@destack/message) | Send messages on a channel, email, push, webhook or a desktop. | Standard Webhooks |
| [`@destack/notification`](@destack/notification) | Declare notifications, notify recipients, watch objects and deliver them as messages. | Knock, Novu |
| [`@destack/social`](@destack/social) | Attach comments, reactions, receipts, favourites and presence to any object, notifying mentioned and participating principals. | Liveblocks |
| [`@destack/observability`](@destack/observability) | Query, follow and alert on the telemetry and analytics of Destack scopes, group their failures into issues, and measure what their spaces use. | Sentry, Grafana |
| [`@destack/finance`](@destack/finance) | Sell products with features, bill accounts for subscriptions and usage, and derive their quotas. | Autumn, Orb |
| [`@destack/vault`](@destack/vault) | Keep secrets as encrypted versions and rotate them without redeploying. | AWS Secrets Manager |
| [`@destack/forge`](@destack/forge) | Host the Git repositories of accounts and publish, resolve and distribute their Destack packages. | GitHub, npm |

## 04 Build

How packages are built, tested and published.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/build`](@destack/build) | Compile Destack packages into one module output per runtime, and keep the builds by content. | Vite, Rolldown, OCI registries |
| [`@destack/git`](@destack/git) | Declare Git storage in Destack and keep its repositories locally or in Cloudflare Artifacts. | — |
| [`@destack/registry`](@destack/registry) | Publish and resolve packages, releases and tags, and push builds and Git, through the registry a space installs. | npm registry |
| [`@destack/test`](@destack/test) | Write and run Destack tests: unit tests on Vitest, simulated worlds, and scenarios played through drivers. | Vitest, Playwright |
| [`@destack/check`](@destack/check) | Lint, format and check Destack packages and their READMEs with one command. | Oxlint |
| [`@destack/sandbox`](@destack/sandbox) | Launch processes with operating system filesystem and network restrictions. | Seatbelt, bubblewrap |
| [`@destack/update`](@destack/update) | Update installed Destack distributions from a signed release feed. | TUF |

## 05 View

What people see and touch.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/style`](@destack/style) | Style components with StyleX, compiled into one stylesheet per build. | StyleX, Tailwind |
| [`@destack/theme`](@destack/theme) | Theme views with a page, an accent and ten hues, solved for contrast in light and dark. | Radix Themes, shadcn/ui |
| [`@destack/ui`](@destack/ui) | Build views from accessible Solid components styled with StyleX and themed by the person's theme. | shadcn/ui, Radix, Base UI |
| [`@destack/view`](@destack/view) | Build a package's views with Solid, reading and changing its space's objects live. | Solid |
| [`@destack/shader`](@destack/shader) | Draw fragment shader effects in views, colored by the theme. | Paper Shaders |

## 06 Object

What apps model, share and sync, and the accounts people sign in with.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/object`](@destack/object) | Declare object types with access, methods and sync, and call them from anywhere. | Convex, oRPC, OpenAPI |
| [`@destack/sync`](@destack/sync) | Synchronize database rows to subscribers and local replicas, as each commit lands. | PostgreSQL logical replication, Replicache |
| [`@destack/access`](@destack/access) | Decide who may do what to each object, and share, delegate and revoke that access. | Zanzibar, SpiceDB |
| [`@destack/event`](@destack/event) | Keep the append-only events of scopes, hot in their database and sealed into their bucket, and query, fold and follow them. | ClickHouse |
| [`@destack/setting`](@destack/setting) | Declare, place and resolve scoped settings. | — |
| [`@destack/account`](@destack/account) | Sign people and machines in, and manage their accounts, organizations and spaces. | Better Auth |

## 07 Storage

Where bytes rest: tables and files.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/db`](@destack/db) | Declare tables, query them on SQLite and PostgreSQL, and log every commit. | Drizzle |
| [`@destack/bucket`](@destack/bucket) | Store files in buckets on local disk, Cloudflare R2 or any S3 endpoint. | Cloudflare R2, S3 |

## 08 Core

What every package uses: schemas, errors, effects, messages, telemetry, mail and identities.

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/schema`](@destack/schema) | Define Destack's portable values: validation schemas, identifiers, versions, time values, digests and canonical JSON. | Zod, JSON Schema |
| [`@destack/error`](@destack/error) | Declare the errors a package's callers receive, each with a stable reason, an HTTP code and typed data. | gRPC status codes |
| [`@destack/environment`](@destack/environment) | Take every effect through one environment: time, randomness and the network, real or simulated. | FoundationDB simulation, TigerBeetle VOPR |
| [`@destack/icon`](@destack/icon) | Name Phosphor icons and store their SVG bodies, each one loaded only where it is used. | Phosphor, Lucide |
| [`@destack/color`](@destack/color) | Read, change and write colors, and name the hues they mean things in. | Radix Colors |
| [`@destack/code`](@destack/code) | Parse, compare, patch and merge source code in the browser, in Bun and in workers. | Lezer, CodeMirror, git |
| [`@destack/locale`](@destack/locale) | Localise messages and format numbers, money and dates for a person's language and region. | Lingui, MessageFormat 2 |
| [`@destack/telemetry`](@destack/telemetry) | Instrument Destack packages with traces, metrics, logs, exceptions and analytics. | OpenTelemetry, Sentry |
| [`@destack/mail`](@destack/mail) | Send email over SMTP or Amazon SES, or print it on machines without a provider. | — |
| [`@destack/package`](@destack/package) | Describe Destack packages, their declarations, builds and source code. | Deno permissions, Storybook |
| [`@destack/resource`](@destack/resource) | Declare the resources a package uses, bind their clients in a workload, and plan their changes. | Kubernetes CRDs, Terraform |
| [`@destack/identity`](@destack/identity) | Prove who a principal is and keep the keys proving it. | did:plc, JWK, DPoP |

<div align="center">

## The Destack

<p>
    <a href="package.json"><img src="https://img.shields.io/badge/version-2026.9.0-2ea44f?style=for-the-badge" alt="Version"></a>
    <a href="LICENSE.txt"><img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="MIT License"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/destack-sh/destack/release.yml?branch=main&label=Release&logo=github&style=for-the-badge" alt="Release"></a>
    <a title="Discord" target="_blank" href="https://discord.gg/xUFQ45TWYd"><img alt="Chat with Destack people on Discord" src="https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white&style=for-the-badge"></a>
</p>

</div>

Destack is an open-source software engine for general purpose application software (i.e. "personal software"), just like Unity and Unreal are engines for games.
Think GitHub + NPM + Convex + Sentry in one box, with all the boring integration already done: auth, schemas, migrations, multiplayer, permissions, workflows, telemetry and hosting.

The full Destack comprises both the _software engine_ for building and running application software _and_ first-party "p90" apps for classic SaaS purposes, like Notion or Linear.
All apps built on Destack - ours and yours - get the advantages of standardisation and integration out of the box "for free!, and are thus fully programmable by your agents.

You can even run _your_ entire Destack - your own personal superapp - on a single SQLite instance on a CLoudflare worker.
Or you can let us host it for you. 
Or any combination!
Moving in is a single prompt, moving out is a single prompt.
Try Destack with:

```bash
curl -fsSL https://destack.sh/install | sh
```

Read more on [destack.sh](https://destack.sh): the [docs](https://destack.sh/docs/), the [hosted pricing](https://destack.sh/pricing/) and the [blog](https://destack.sh/blog/).

## The Destack

Destack is mostly written in itself using TypeScript and other modern boring Web technologies, so the actual "stack" is composed with [incremental granularity](https://caseymuratori.com/blog_0016) as classic packages:

### 01 Users

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/identity`](@destack/identity) | Create self-certifying identities with signing and rotation keys, and keep keys in keyrings and keychains | did:plc, JWK, DPoP |
| [`@destack/account`](@destack/account) | Sign people in with magic links, passkeys, two-factor and OAuth, and serve accounts, spaces, domains and an OpenID Connect provider | Better Auth |
| [`@destack/connection`](@destack/connection) | Store credentials for external accounts and attach them to outbound requests | Nango |
| [`@destack/finance`](@destack/finance) | Declare products, features and metered usage, bill through a payment provider, and derive entitlements | Autumn, Orb |

### 02 Apps

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/style`](@destack/style) | Write component styles in StyleX, with Tailwind's breakpoints and preflight | StyleX, Tailwind |
| [`@destack/theme`](@destack/theme) | Define themes over colour roles, with contrast, nested surfaces, swatches and motion | Radix Themes, shadcn/ui |
| [`@destack/icon`](@destack/icon) | Draw icons by name, loaded lazily | Phosphor |
| [`@destack/view`](@destack/view) | Render package views in one shell, and give them live queries, mutations, settings and commands | Solid |
| [`@destack/shader`](@destack/shader) | Render animated gradients and effects as Solid components | Paper Shaders |
| [`@destack/ui`](@destack/ui) | Use buttons, dialogs, selects, sidebars, charts and the rest on Solid and StyleX | shadcn/ui, Radix, Base UI |
| [`@destack/web`](@destack/web) | Build websites with server rendering, prerendering, and sitemap, robots and manifest files | SolidStart, Next.js |
| [`@destack/desktop`](@destack/desktop) | Run the native shell: windows, menus, banners, the badge, the tray and the command palette | Tauri |
| [`@destack/cli`](@destack/cli) | Read and change a space's objects, stacks, installations and packages from the terminal | kubectl, gh |

### 03 Services

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/error`](@destack/error) | Declare the call errors callers receive, with their codes and HTTP statuses | gRPC status codes |
| [`@destack/telemetry`](@destack/telemetry) | Emit logs, traces and metrics, and capture errors and sessions, from any package | OpenTelemetry, Sentry |
| [`@destack/locale`](@destack/locale) | Write translatable messages and format them for a locale from catalogs | Lingui, MessageFormat 2 |
| [`@destack/environment`](@destack/environment) | Take time and the network as effects, real or simulated with virtual time and faults | FoundationDB simulation, TigerBeetle VOPR |
| [`@destack/access`](@destack/access) | Declare relations and permissions as a schema, and check, grant and request access | Zanzibar, SpiceDB |
| [`@destack/runtime`](@destack/runtime) | Declare procedures with authentication and permissions, serve them over oRPC with OpenAPI routes, and issue call tokens | oRPC, OpenAPI |
| [`@destack/setting`](@destack/setting) | Declare typed settings and resolve each value along user, space, installation and device scopes | — |
| [`@destack/workflow`](@destack/workflow) | Run durable functions that record each step, with schedules, triggers and approvals | Temporal, DBOS, Inngest, Restate |
| [`@destack/message`](@destack/message) | Send email over SMTP, web push, webhooks and desktop messages, and sign webhook deliveries | Standard Webhooks |
| [`@destack/notification`](@destack/notification) | Record notifications per recipient, keep each person's inbox, and send them as messages under focus and summary settings | Knock, Novu |
| [`@destack/social`](@destack/social) | Add comments, reactions, read receipts, favourites and presence to any object type | Liveblocks |
| [`@destack/observability`](@destack/observability) | Store a space's logs, spans and metrics, group failures into issues, fire alerts, and declare usage meters | Sentry, Grafana |

### 04 Data

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/schema`](@destack/schema) | Declare schemas limited to what JSON Schema expresses, with Temporal time types, and convert them both ways | Zod, JSON Schema |
| [`@destack/db`](@destack/db) | Define and query SQL tables on SQLite and PostgreSQL, with migrations, full-text search, sealed columns and a change log | Drizzle |
| [`@destack/sync`](@destack/sync) | Replicate queried rows from the change log to other databases and to browsers, with optimistic local writes | PostgreSQL logical replication, Replicache |
| [`@destack/bucket`](@destack/bucket) | Store files through the R2 binding API, in local directories, in R2 or in any S3-compatible store | Cloudflare R2, S3 |
| [`@destack/event`](@destack/event) | Append events to a table, compact them into Parquet segments, and query, aggregate and delete them | ClickHouse |
| [`@destack/object`](@destack/object) | Declare object types: synced tables with access rules, methods, history and events | Convex |
| [`@destack/vault`](@destack/vault) | Store versioned secrets encrypted at rest, generate keys, and stage and promote versions | AWS Secrets Manager |

### 05 Source

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/package`](@destack/package) | Describe a package in `destack.json`: its sandbox permissions, examples and scenarios, manifests and module transform | Deno permissions, Storybook |
| [`@destack/resource`](@destack/resource) | Declare resource kinds with desired state, and plan, apply and upgrade them through providers | Kubernetes CRDs, Terraform |
| [`@destack/test`](@destack/test) | Run tests and play declared scenarios through a driver per interaction | Vitest, Playwright |
| [`@destack/build`](@destack/build) | Compile a package into one output per runtime, and store builds by content | Vite, Rolldown, OCI registries |
| [`@destack/stack`](@destack/stack) | Declare a space's resources and package installations, and apply them as revisions with a plan and an approval | Pulumi, CloudFormation |
| [`@destack/forge`](@destack/forge) | Host Git repositories and publish packages as npm releases | GitHub, npm |

### 06 Hosts

| Package | What it does | Prior art |
|---|---|---|
| [`@destack/sandbox`](@destack/sandbox) | Run processes with explicit filesystem and network permissions on macOS and Linux | Seatbelt, bubblewrap |
| [`@destack/update`](@destack/update) | Check, download, stage and activate signed releases, and publish them | TUF |
| [`@destack/relay`](@destack/relay) | Route requests for a space's or a machine's name through a tunnel to the machine serving it | Cloudflare Tunnel, ngrok |
| [`@destack/machine`](@destack/machine) | Run spaces on any host, on Bun or Cloudflare, with placement, builds, a request gateway and metering | — |
| [`@destack/universe`](@destack/universe) | Run the account, setting and directory packages over one database, on Bun or as a Cloudflare Worker | — |

## Contributing

Destack is not currently open for public source contributions, but we welcome feedback, issues, and ideas.

## License

Destack is fully open source under the MIT license across the full stack (see [LICENSE.txt](LICENSE.txt)).

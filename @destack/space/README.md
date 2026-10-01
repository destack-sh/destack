Declare and serve spaces: their resources, installations, stacks, policies, roles and relationships.

## Declaring a space

A stack exports space definitions that declare resources and install packages without provisioning anything.

```ts
import { defineSpace, install } from "@destack/space";
import { database } from "@template/stack";
import notes from "@destack/notes/package";

export const personal = defineSpace({
    resources: { main: { declaration: database, retention: "retain", tags: {} } },
    installations: { notes: install(notes, { main: "main" }) },
});
```

A parameterised export takes explicit values and exports its input schema.

```ts
export const Parameters = defineSchema(schema.object({ alias: schema.string().min(1) }));

export function preview(input: schema.Infer<typeof Parameters>) {
    const { alias } = Parameters.parse(input);

    return defineSpace({ installations: { notes: install(notes, { main: "main" }, { alias }) } });
}
```

## Policies

A space declares package admission and outbound connections independently.

```ts
export const restricted = defineSpace({
    policies: {
        packages: { admission: { default: "deny", rules: { destack: { package: { kind: "destack" }, decision: "allow" } } } },
        network: {
            default: "deny",
            rules: {
                api: {
                    destination: { kind: "hostname", hostname: "api.example.com", subdomains: false },
                    protocol: "https",
                    decision: "allow",
                },
            },
        },
    },
});
```

## Roles and relationships

Relationships refer to objects, installations and roles by their key in the stack or by their identifier in the same space.

```ts
export const shared = defineSpace({
    roles: { editor: { description: "Edit notes", permissions: [note.permission("update")] } },
    relationships: {
        notes: { subject: { installation: "notes" }, role: "editor" },
        members: { subject: { accountMembers: true }, role: { id: "role-01995688-0000-7000-8000-000000000001" } },
    },
});
```

## Submitting and approving

An installation follows the builds submitted to it, and its controller applies each one.

| Object | Plans | Waits with |
|---|---|---|
| stack installation | its declarations | `plan`, `AwaitingApproval` |
| application installation | the upgrade steps from its applied release | `plan`, `AwaitingApproval` |
| resource | its provider's plan, recreating over draining releases when they conflict | `status.state.plan`, `AwaitingApproval` |

A plan waits once its risk reaches the space's `approval` threshold, and `installation.plan` joins what an installation and what it manages, owns or binds wait on.

```ts
await client.installation.submit({ spaceId, id: stack.id, requestId, selection, build, evaluation });
const waiting = await client.installation.plan({ spaceId, id: stack.id });
await client.installation.approve({ spaceId, id: stack.id, requestId, plan: await Plan.digest(waiting) });
```

The CLI submits a checkout's stack and approves what waits with `--yes`.

```sh
destack stack apply --space work.acme --export personal --yes
destack stack approve --space work.acme --plan <digest>
```

## Views

A client opens a view the revision of an installation declares.

```ts
const { build, definition } = await client.installation.open({ spaceId, id, view: "home", path: "/" });
```

## Workloads

Each workload runs on Bun from its own `./workload/<name>` entry with the resources its deployment captured.

```ts
import { run } from "@destack/space/runner";

await run(workload, { main: database });
```

## Runs and schedules

A run is one object method call that an installation's cell makes later, recorded once per cause.

| Cause | Recorded by | Once per |
|---|---|---|
| `send` | the workload, through `context.runs` or a method's outbox | request |
| `schedule` | the cell, from a schedule's timing, at most once a minute | occurrence |
| `webhook` | the workload, after verifying the delivery | route path and delivery |
| `watch` | the workload's object server, following its log | log position and row |

The cell pushes each run through the host's router as the installation or on the authority lent to it.

```ts
const service = implementService({ ...options, runs: { push: pushThrough(router), verify: (token) => lending.verify(token) } });
```

## Serving spaces

A host or region serves its spaces' objects, controllers and zone relay through `implementService`.

```ts
import { implementService } from "@destack/space/server";

const service = implementService({
    database,
    audit,
    global: { directory, replicas, relay: (cell) => peer(cell).zone },
    cell: { hostId },
    served: [accountId],
    providers,
    declared: [],
    openBuild: (packageId, build) => builds.open(packageId, build),
    bindables: [],
    runtimes,
    runs: { push: pushThrough(router), verify: (token) => lending.verify(token) },
});
```

## Branches

Every app serves `branchObjects` beside its own objects and keeps `branchTables` in its database; `branchType` brings them together for servers and clients.

```ts
export const notesService = defineService("notes", { objects: { notebook, note, ...branchObjects } });
```

| Method | Permission | Effect |
|---|---|---|
| `branch.create` | `push` | create a branch under a title, readable by its author |
| `branch.push` | `push` | append calls as the caller's, and replay them into the branch's rows |
| `branch.merge` | `merge` | replay every call with the merger's permission, each as its author, in one mutation |
| `branch.discard` | `discard` | close the branch without replaying it |
| `branch.rebuild` | system | replay every call again once the main line changes a table the branch reaches |
| `branch.grant` | `share` | let a collaborator read, push and merge |

## Transfers

A space moves to a host of its account or to a region with every row its cell database keeps in the space and its resources.

```ts
await client.transfer.create({ spaceId, requestId, target: hostId });
```

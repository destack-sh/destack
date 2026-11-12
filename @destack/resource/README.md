# @destack/resource

Declare the resources a package uses, bind their clients in a workload, and plan their changes.

## Overview

`@destack/resource` is infrastructure as packages declare it: resource declarations, clients bound per call, and providers that plan and apply changes.

Your package declares its resources and reads their clients:

- `ResourceDeclaration`: one resource of a kind, such as a bucket or a database, with its spec.
- `ResourceContext`: the clients bound for one call, like a Worker's `env`.
- `Secret` and `ExternalClient`: a versioned credential, and an API behind the host's egress.

Hosts manage resources:

- `Provider`: manages one kind's resources on a host, like a Terraform provider.
- `Plan`: the steps that change resources, each with a risk.

Declare a queue, bind its client, and use it in a workload:

```ts
import { defineKind } from "@destack/package";
import { ResourceDeclaration } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";

/** A queue's client. */
interface Queue {
    readonly send: (body: string) => Promise<void>;
}

// declare a kind of queue, and this package's jobs queue
const QueueKind = defineKind({ name: "queue", provisioned: { spec: schema.object({}) } });
export const jobs = new ResourceDeclaration<Queue>(import.meta.destack.package, QueueKind, {
    name: "jobs",
    spec: {},
});

// bind the client, then send a job
const context = new ResourceContext().bind(jobs, { send: async () => undefined });
await jobs.get(context).send("resize photo-1");
```

## Reference

### ResourceDeclaration

`new ResourceDeclaration(owner, kind, { name, spec })` declares a resource.

| Verb                                              | What it does                                                                   |
| ------------------------------------------------- | ------------------------------------------------------------------------------ |
| `get(context)`                                    | reads the client bound to the current call                                     |
| `provision(options)`                              | describes the resource a stack creates, with its placement, retention and tags |
| `describe()`                                      | describes the declaration as its kind does: its name and spec                  |
| `state()`                                         | describes the state the resource must hold                                     |
| `toJSON()`, `ResourceDeclaration.fromJSON(value)` | writes and reads the declaration as JSON                                       |

`provision` takes these options:

| Option      | What it is                                                                              |
| ----------- | --------------------------------------------------------------------------------------- |
| `placement` | `{ provider, location, machine }`, where the resource lives                             |
| `retention` | `"delete"`, `"forever"` or `{ within: duration }`, what deleting it does to its content |
| `tags`      | labels by name                                                                          |

### ResourceContext

A host binds each client with `bind(resource, client)`, and `get(resource)` reads it.

### Plan

A plan is `{ steps, deferred }`: its steps in order, and why it stops early.
A step is `{ action, target, risk, detail, fields }`, with `fields` before and after.

| Risk                    | What the step can do                                 |
| ----------------------- | ---------------------------------------------------- |
| `safe`                  | nothing a reader notices                             |
| `fallible`              | fail on existing content, such as a new unique index |
| `backward-incompatible` | break readers of the previous release                |
| `destructive`           | lose content                                         |
| `privileged`            | grant access                                         |

Actions are `create`, `update`, `delete`, `replace`, `rename`, `convert`, `restore` and `grant`.

| Verb                         | What it does                                                      |
| ---------------------------- | ----------------------------------------------------------------- |
| `Plan.classify(plan)`        | reads its highest risk, `safe` when empty                         |
| `Plan.isAtLeast(plan, risk)` | reports whether its highest risk is at least a risk               |
| `Plan.join(plans)`           | joins plans in order, collecting every problem                    |
| `Plan.digest(plan, crypto)`  | digests its steps, so an apply runs exactly the plan a review saw |

### Provider

Each part of a provider is optional.

| Member         | What it does                                                         |
| -------------- | -------------------------------------------------------------------- |
| `kind`, `code` | the kind it manages, and its code on resources, such as `sqlite`     |
| `reconcile`    | plans and applies the steps to a resource's desired state            |
| `provision`    | creates and destroys the resources it hosts                          |
| `open`         | opens a resource's content as a database                             |
| `fence`        | refuses writes while a transfer copies the content                   |
| `snapshot`     | copies the content at a point in time into a content-addressed store |
| `rewrap`       | rewraps the rows bound to this host for another host                 |

### Secret

A secret has `read()` for its current version, and `versions()` for every readable one, newest first.
`Secret.of(value)` makes a one-version secret from a value, such as an environment variable.

## Examples

### Provision a resource

`provision` describes the resource a stack creates: where it lives, and what deleting it does.

```ts
import { defineKind } from "@destack/package";
import { ResourceDeclaration } from "@destack/resource";
import { schema } from "@destack/schema";

// declare a queue kind and this package's jobs queue
const QueueKind = defineKind({ name: "queue", provisioned: { spec: schema.object({}) } });
const jobs = new ResourceDeclaration(import.meta.destack.package, QueueKind, {
    name: "jobs",
    spec: {},
});

// place it on a provider, kept 30 days after deletion
export const provisioned = jobs.provision({
    placement: { provider: "local" },
    retention: { within: { days: 30 } },
    tags: { team: "billing" },
});
```

### Write a provider

A provider with only `provision` creates and destroys one kind's resources, here as directories.

```ts
import { join } from "node:path";
import type { FileSystem } from "@destack/environment";
import { defineKind } from "@destack/package";
import type { Provider, ResourceRecord } from "@destack/resource";
import { schema } from "@destack/schema";

// declare the kind the provider manages
const QueueKind = defineKind({ name: "queue", provisioned: { spec: schema.object({}) } });

/** Keep each queue as a directory below a host directory. */
export function directoryProvider<Object>(
    directory: string,
    object: Object,
    files: FileSystem,
): Provider<typeof QueueKind, Object> {
    // place each resource below the directory by its identifier
    const path = (resource: Pick<ResourceRecord, "id">) => join(directory, resource.id);

    return {
        kind: QueueKind,
        code: "directory",
        object,
        provision: {
            provision: async (resource) => {
                await files.mkdir(path(resource), { recursive: true });

                return { reference: resource.id };
            },
            destroy: (resource) => files.remove(path(resource), { recursive: true, force: true }),
        },
    };
}
```

### Call an external API

`fetch` calls a path below the API's address, and the egress adds the credential.

```ts
import type { ExternalClient } from "@destack/resource";

/** List the customers of a payment API. */
export async function listCustomers(api: ExternalClient): Promise<unknown> {
    const response = await api.fetch("/v1/customers");

    return await response.json();
}
```

### Review a plan

`Plan.classify` reads a plan's risk, and `Plan.digest` names the exact steps an approval covers.

```ts
import { Plan } from "@destack/resource";
import { BunEnvironment } from "@destack/environment/bun";

// plan a new column and a dropped one
const plan: Plan = {
    steps: [
        {
            action: "create",
            target: "table/note/column/priority",
            risk: "safe",
            detail: "add column priority",
        },
        {
            action: "delete",
            target: "table/note/column/color",
            risk: "destructive",
            detail: "drop column color",
        },
    ],
};

// read its risk and digest
const risk = Plan.classify(plan);
const digest = await Plan.digest(plan, BunEnvironment.system.crypto);
```

## Comparison

### Terraform and Kubernetes

Kinds basically follow Kubernetes' custom resources, and providers follow Terraform's.
Every plan step has a risk, and risky plans wait for approval.

| Prior art                  | `@destack/resource`           | Difference                                                         |
| -------------------------- | ----------------------------- | ------------------------------------------------------------------ |
| a CustomResourceDefinition | `defineKind({ provisioned })` | Declared by a package for the packages that use it.                |
| a Terraform provider       | `Provider`                    | Also opens, fences and snapshots content for transfers.            |
| `terraform plan`           | `Plan`                        | Each step has a risk, and the space approves a plan by its digest. |
| a Worker's `env` bindings  | `ResourceContext`             | The same, bound per call.                                          |

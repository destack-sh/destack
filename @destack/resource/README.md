# @destack/resource

Declare the resources a package uses, bind their clients in a workload, and plan their changes.

## Overview

A package declares the resources it uses by kind, a stack provisions them, and the host binds each declaration to a client when a workload runs.

- **ResourceDeclaration**: one resource a package uses, of a kind such as a bucket or a database, with its spec; `provision` describes the resource a stack creates, and `get` returns the bound client.
- **ResourceContext**: the clients a host binds for one invocation, as a Worker's `env` keeps its bindings; `bind` and `get`.
- **Provider**: what plans, applies, provisions, snapshots and fences one kind's resources on a host, as a Terraform provider does.
- **Plan**: the steps that change resources, each with a risk from `safe` to `privileged`; a plan at or above `backward-incompatible` needs approval.
- **Secret**: a credential a package reads by version.

```ts
import { defineKind } from "@destack/package";
import { Plan, ResourceDeclaration } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";

interface Queue {
    readonly send: (body: string) => Promise<void>;
}
const QueueKind = defineKind({ name: "queue", provisioned: { spec: schema.object({}) } });
const owner = import.meta.destack.package;
const jobs = new ResourceDeclaration<Queue>(owner, QueueKind, { name: "jobs", spec: {} });

const created = jobs.provision({ retention: { within: { days: 30 } } });
const context = new ResourceContext().bind(jobs, { send: async () => undefined });
await jobs.get(context).send("hello");
const plan: Plan = {
    steps: [{ action: "create", target: "resource/jobs", risk: "safe", detail: "create jobs" }],
};
Plan.isAtLeast(plan, "backward-incompatible"); // false
```

## Examples

### Use an operator's value as a secret

`Secret.of` turns a value an operator sets, such as an environment variable, into a secret with one version.

```ts
import { Secret } from "@destack/resource";

const stripe = Secret.of("sk_test_…");
const { value } = await stripe.read();
```

### Call an external API

A connection's client calls the API below its bound address, the host's egress adding the credential.

```ts
import type { ExternalClient } from "@destack/resource";

async (api: ExternalClient) => {
    const response = await api.fetch("/v1/customers");
};
```

## Comparison

### Kubernetes custom resources

Kinds with a spec and desired state follow Kubernetes' CustomResourceDefinitions, planned and applied as Terraform does; three things differ.

- A package declares only what it uses, and the stack that installs it decides where each resource lives.
- Every plan step has a risk, and a risky plan waits for the space's managers to approve it.
- A workload reads its clients from a `ResourceContext`, never from globals or environment variables.

# @destack/observability

Query, follow and alert on the telemetry and analytics of Destack scopes, group their failures into issues, and measure what their spaces use.

## Overview

The observability installation keeps a space's logs, spans, metrics, sessions, visits and actions, groups its failures into issues and fires alerts on them.

- **Event types**: `log`, `span`, `metric`, `session`, `visit` and `action`, their personal attributes masked unless the reader may unmask them.
- **Exception**: one failure a log record or span reports, named into its issue by installation and fingerprint.
- **Issue**: failures grouped by fingerprint; `resolve`, `ignore` and assign, reopened when it fails again.
- **Alert rule**: a condition over issues or folds of events, with its actions; an `Alert` fires once its condition lasts its pending time, unless a `Silence` covers it.

```ts
import type observabilityPackage from "@destack/observability/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    issue: Identifier<"issue">,
    requestId: string,
) => {
    const exceptions = await client.exception.list({ spaceId, where: { issue } });
    const logs = await client.log.list({ scope: spaceId, where: { session: "session-1" } });
    await client.issue.resolve({ spaceId, requestId, id: issue, until: { kind: "never" } });
};
```

## Examples

### Install observability

A space installs observability, and its machine forwards every installation's telemetry to it.

```sh
destack install @destack/observability
```

### Send telemetry over OTLP

The `otlp` mount takes OTLP/HTTP JSON exports at `/v1/traces`, `/v1/logs` and `/v1/metrics`.

```ts
async (origin: string, token: string, body: string) => {
    const headers = { authorization: `Bearer ${token}`, "content-type": "application/json" };
    await fetch(`${origin}/v1/logs`, { method: "POST", headers, body });
};
```

### Alert on an error rate

An alert rule over events folds them per window and fires once the fold stays above its threshold for the pending time.

```ts
import type observabilityPackage from "@destack/observability/package";
import { log } from "@destack/observability/object";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    requestId: string,
) => {
    await client.alertRule.create({
        spaceId,
        requestId,
        title: "Error rate",
        condition: {
            kind: "events",
            event: log.typeReference,
            where: "severity >= 17",
            fold: "count",
            group: ["installationId"],
            per: {},
            window: { minutes: 5 },
            comparison: "above",
            threshold: 50,
        },
        actions: [{ kind: "notify" }],
        pending: { minutes: 2 },
        noData: "ok",
    });
};
```

### Silence alerts

A silence mutes the alerts whose labels match, between its start and end.

```ts
import type observabilityPackage from "@destack/observability/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    requestId: string,
    startsAt: number,
    endsAt: number,
) => {
    await client.silence.create({
        spaceId,
        requestId,
        matchers: [{ label: "installationId", operator: "=", value: "notes" }],
        startsAt,
        endsAt,
        comment: "maintenance",
    });
};
```

## Comparison

### Sentry

Issues, fingerprints, regressions and release health follow Sentry; three things differ.

- Telemetry arrives over OTLP from every installation of a space, kept in the space's event store.
- Personal attributes stay masked unless the reader may unmask them.
- Issues are objects of the space, so their access, audit and sync are any object's.

### Prometheus Alertmanager

Alert rules, pending alerts and silences follow Prometheus and its Alertmanager; two things differ.

- A rule's condition folds the space's events or watches its issues, never a separate metrics store.
- An alert's actions can call an installation's method, such as a rollback, as the rule's author.

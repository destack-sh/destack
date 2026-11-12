# @destack/observability

Query, follow and alert on the telemetry and analytics of Destack scopes, group their failures into issues, and measure what their spaces use.

## Overview

`@destack/observability` is Sentry's issues and Alertmanager's alerts over a space's event store: telemetry events, issues, alert rules and silences.

Spaces keep telemetry as events:

- `log`, `span`, `metric` and `session`: telemetry.
- `visit` and `action`: analytics.

Failures group into issues:

- `exception`: one reported failure.
- `issue`: exceptions with one fingerprint.

Alerts:

- `alertRule`: a condition over issues or events, and its actions.
- `alert`: one subject crossing a rule's condition.
- `silence`: a window that mutes matching alerts.

Read an issue's exceptions and its session's logs, then resolve it:

```ts
import type { ClientOf } from "@destack/object/client";
import type observabilityPackage from "@destack/observability/package";
import type { Identifier } from "@destack/schema";

/** Read what an issue recorded, then resolve it until it fails again. */
export async function triage(
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    issue: Identifier<"issue">,
    requestId: string,
) {
    // read the issue's exceptions and a session's logs
    const exceptions = await client.exception.list({ spaceId, where: { issue } });
    const logs = await client.log.list({ scope: spaceId, where: { session: "session-1" } });

    // resolve it
    await client.issue.resolve({ spaceId, requestId, id: issue, until: { kind: "event" } });

    return { exceptions, logs };
}
```

## Examples

### Alert on an error rate

This rule fires when error logs stay above 50 per five minutes for two minutes.

```ts
import type { ClientOf } from "@destack/object/client";
import { log } from "@destack/observability/object";
import type observabilityPackage from "@destack/observability/package";
import type { Identifier } from "@destack/schema";

/** Alert on an installation logging more than 50 errors in five minutes. */
export async function alertOnErrors(
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    requestId: string,
) {
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
}
```

### Roll back on a regression

An issue rule fires once per opened or regressed issue, and a `call` action calls the installation's method.

```ts
import type { ClientOf } from "@destack/object/client";
import type observabilityPackage from "@destack/observability/package";
import type { Identifier } from "@destack/schema";

/** Notify the managers and roll back an installation whose fixed issue fails again. */
export async function rollBackOnRegression(
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    requestId: string,
) {
    await client.alertRule.create({
        spaceId,
        requestId,
        title: "Roll back on regression",
        condition: { kind: "issue", on: "regress" },
        actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
    });
}
```

### Ignore an issue until it grows

`ignore` and `resolve` take an `until`: the next event, a newer revision, a time, a count, or never.

```ts
import type { ClientOf } from "@destack/object/client";
import { issue, type Issue } from "@destack/observability/object";
import type observabilityPackage from "@destack/observability/package";

/** Ignore a known issue until it has happened a hundred times. */
export async function ignoreUntilHundred(
    client: ClientOf<typeof observabilityPackage>,
    known: Issue,
    requestId: string,
) {
    await client.issue.ignore({
        ...issue.target(known),
        requestId,
        until: { kind: "count", count: 100 },
    });
}
```

### Silence alerts during maintenance

A silence mutes matching alerts between its start and end.

```ts
import type { ClientOf } from "@destack/object/client";
import type observabilityPackage from "@destack/observability/package";
import type { Identifier } from "@destack/schema";

/** Mute the notes installation's alerts for an hour of maintenance. */
export async function silenceNotes(
    client: ClientOf<typeof observabilityPackage>,
    spaceId: Identifier<"space">,
    requestId: string,
    now: number,
) {
    await client.silence.create({
        spaceId,
        requestId,
        matchers: [{ label: "installationId", operator: "=", value: "notes" }],
        startsAt: now,
        endsAt: now + 3_600_000,
        comment: "maintenance",
    });
}
```

## Comparison

### Sentry

Issues, fingerprints, regressions, resolving until a release and release health basically follow Sentry.
Issues are objects of the space.

| Sentry                                | `@destack/observability`         | Difference                                                     |
| ------------------------------------- | -------------------------------- | -------------------------------------------------------------- |
| an issue, its fingerprint             | `issue`                          | One per installation and fingerprint.                          |
| resolve in next release, ignore until | `resolve`, `ignore` with `until` | The same, by revision, time or count.                          |
| regression                            | `"regressed"`                    | The same.                                                      |
| release health sessions               | `session`                        | `ok`, `errored` or `crashed`, by build.                        |
| the SDK's envelope intake             | the `otlp` mount                 | OTLP from every installation, kept in the space's event store. |

### Prometheus Alertmanager

Alert rules, pending alerts and silences basically follow Prometheus and its Alertmanager.
Rule actions can call an installation's method.

| Prometheus                   | `@destack/observability`   | Difference                                                |
| ---------------------------- | -------------------------- | --------------------------------------------------------- |
| an alerting rule with `for`  | `alertRule` with `pending` | Folds events from the space's store, not a metrics store. |
| pending, firing, resolved    | the same                   | The same.                                                 |
| silences with label matchers | `silence` with `matchers`  | `=` and `!=`, without regular expressions.                |
| receivers                    | `notify`, `call`           | Notifies subscribers, or calls a rollback.                |

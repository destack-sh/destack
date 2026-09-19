Define Destack HTTP services.

## Usage

```ts
import { schema } from "@destack/schema";
import { procedure } from "@destack/service";
import { describeService } from "@destack/service/inspect";

const notes = {
    list: procedure.route({ method: "GET", path: "/notes" })
        .output(schema.array(schema.object({ id: schema.string(), title: schema.string() }))),
};

const description = await describeService("notes", notes, {
    info: { title: "Notes", version: "2026.9.0" },
});
```

Handle declared errors through the typed client.

```ts
import { safe } from "@destack/service/client";

const result = await safe(client.update(input));
if (result.isDefined && result.error.code === "CONFLICT") {
    console.log(result.error.data.revision);
} else if (result.error) {
    throw result.error;
}
```

Unexpected handler failures are logged through `@destack/telemetry` and returned as internal errors.

## Workloads

Declare the HTTP handler referenced by a workload.

```ts
import { defineService } from "@destack/service";

export const web = defineService({ name: "web", version: 1, protocol: "http", handler: "fetch" });

export function fetch(request: Request): Response {
    return Response.json({ path: new URL(request.url).pathname });
}
```

## Schedules

```ts
import { defineSchedule } from "@destack/service/schedule";

export const reminders = defineSchedule({
    name: "reminders",
    version: 1,
    handler: "remind",
    timing: "cron",
    cron: "0 9 * * *",
    timezone: "Europe/Zurich",
    concurrency: "forbid",
    deadline: 60000,
});
```

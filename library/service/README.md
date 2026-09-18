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

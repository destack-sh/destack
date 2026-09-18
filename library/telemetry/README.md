Instrument Destack packages with traces, metrics, and logs using
[OpenTelemetry](https://opentelemetry.io/docs/languages/js/).

## Usage

Create package instruments at module scope.

```ts
import { SpanStatusCode, telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { saveNote } from "./note.ts";

const { tracer, meter, logger } = telemetry.scope(import.meta.destack.package);
const saved = meter.createCounter("notes.saved");

export async function save() {
    return tracer.startActiveSpan("save note", async (span) => {
        try {
            await saveNote({ title: "Hello, Destack!" });
            saved.add(1);
            logger.emit({ body: "Note saved" });
        } catch (error) {
            span.setStatus({ code: SpanStatusCode.ERROR });
            throw error;
        } finally {
            span.end();
        }
    });
}
```

## Initialization

The host initializes providers before loading application modules.

```ts
import { startTelemetry } from "@destack/telemetry/host";

const telemetry = await startTelemetry({
    name: "notes",
    version: "2026.9.0",
    traces: { spanProcessors },
    metrics: { readers },
    logs: { processors },
});

const application = await import("./application.ts");
```

Use `/browser` for browser initialization and `/worker` with the worker's context manager.
Registered providers belong to one application runtime; isolated providers use `telemetry.scope()`.

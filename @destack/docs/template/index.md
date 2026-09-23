---
title: Templates
description: Starting points for new projects.
---

# Templates

Templates are ordinary packages with a `template` declaration in `destack.json`.

- `stack`: shared resource declarations and optional space configuration.
- `blank`: an empty TypeScript package with a selected stack dependency.

## Generation

```ts
import { generateTemplate, readTemplate, writeTemplate } from "@destack/build/template";

const source = await readTemplate("@destack/template-blank");
const files = generateTemplate(source, {
    name: "@florian/notes",
    dependencies: {
        "@destack/template-stack": { name: "@florian/stack", version: "2026.9.0" },
    },
});
await writeTemplate(files, "./notes");
```

Registry consumers supply the verified source files from a selected package release.
Generation updates JSON metadata and parsed module references.
It does not execute template code, install dependencies, or provision resources.

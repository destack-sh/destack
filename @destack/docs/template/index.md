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
import { Template } from "@destack/build/template";

const template = await Template.read("./@template/blank");
await template.write("./notes", {
    id: PackageId.parse(`package-${v7()}`),
    name: "@florian/notes",
    dependencies: {
        "@template/stack": { id: stackId, name: "@florian/stack", version: "2026.9.0" },
    },
});
```

Registry consumers read the verified source files of a selected package release.
Generation rewrites the JSON metadata and the parsed module references, and leaves template code unexecuted, dependencies uninstalled and resources unprovisioned.

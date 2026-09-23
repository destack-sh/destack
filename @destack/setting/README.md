Declare typed settings and resolve authorized assignments and policies.

```ts
// src/settings/editor.ts
import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

export const editorMode = defineSetting({
    package: import.meta.destack.package,
    name: "editor.mode",
    title: "Editor mode",
    description: "Keyboard behavior in the note editor.",
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
    overrides: ["space", "installation", "device"],
    apply: "immediate",
});
```

```ts
// src/stack/settings.ts
import { defineSettingAssignment } from "@destack/setting/declare";
import { editorMode } from "../settings/index.ts";

export const editorAssignment = defineSettingAssignment(
    editorMode,
    {
        kind: "user",
        user: {
            kind: "user",
            authority: "global",
            id: "user-019f5530-8000-7000-8000-000000000003",
        },
    },
    "vim",
);
```

```ts
import type { SettingContext } from "@destack/setting";
import { appearance } from "@destack/theme/settings";

export async function openEditor(settings: SettingContext) {
    const result = await settings.resolve({ editorMode, appearance });

    return { mode: result.editorMode.value, appearance: result.appearance.value };
}
```

```ts
// observe one complete batch until this view closes
for await (const result of settings.watch({ editorMode, appearance }, signal)) {
    renderEditor(result.editorMode.value, result.appearance.value);
}
```

```ts
// construct request-scoped access in service middleware
import { SettingClient } from "@destack/setting/client";

const settings = new SettingClient(
    authenticatedSettingClient,
    context.audience,
    { kind: "user", user: verifiedUser, location: { spaceId: context.spaceId } },
    context.request.signal,
);

return next({ context: { settings } });
```

```ts
import { createSettingClient } from "@destack/setting/client";
import { createRequestId } from "@destack/service/request";

const client = createSettingClient({ url: "https://settings.example.test" });

await client.assignment.set({
    packageId: import.meta.destack.package.id,
    requestId: createRequestId(),
    setting: editorMode.reference,
    target: editorAssignment.target,
    expectedRevision: null,
    value: "vim",
});
```

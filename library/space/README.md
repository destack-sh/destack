Declare resources, installations, permissions, and routes for a space.

## Usage

Stack exports compose space configuration without provisioning resources.

```ts
import { defineSpace, type SpaceDefinition } from "@destack/space";
import { defineDatabase } from "@destack/db/declare";

export const database = defineDatabase({
    name: "main",
    spec: { dialect: "sqlite" },
});

const personal = {
    resources: { main: { declaration: database, retention: "retain", tags: {} } },
    installations: {
        notes: {
            package: { name: "@florian/notes", version: "2026.9.0" },
            alias: "notes",
            state: "enabled",
            resources: { "@florian/stack": { main: { resource: "main" } } },
            secrets: {},
            compute: {},
            tags: {},
        },
    },
} satisfies SpaceDefinition;

export const local = defineSpace(personal);
export const cloud = defineSpace({ ...personal });
```

Independent objects remain managed through the space API. References can select configuration keys
or existing objects in the destination space.

```ts
export const routing = defineSpace({
    routes: {
        pages: {
            domain: "domain-01995688-0000-7000-8000-000000000001",
            path: "/pages",
            match: "prefix",
            destination: {
                installation: {
                    id: "installation-01995688-0000-7000-8000-000000000001",
                },
                entrypoint: ".",
            },
        },
    },
});
```

Parameterised space configuration exports accept explicit values and export their input schema.

```ts
import { defineSchema, schema } from "@destack/schema";

export const Parameters = defineSchema(
    schema.object({ alias: schema.string().min(1) }),
);

export function preview(input: schema.Infer<typeof Parameters>) {
    const parameters = Parameters.parse(input);

    return defineSpace({
        ...personal,
        installations: {
            ...personal.installations,
            notes: { ...personal.installations.notes, alias: parameters.alias },
        },
    });
}
```

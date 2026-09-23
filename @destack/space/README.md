Declare resources, installations, policies, permissions, and routes for a space.

## Usage

Stack exports compose space configuration without provisioning resources.

```ts
import { defineSpace, type SpaceDefinition } from "@destack/space";
import { defineDatabase } from "@destack/db/declare";
import { PackageId } from "@destack/package";
import type {} from "@destack/package/import-meta";

export const database = defineDatabase({
    name: "main",
    spec: { dialect: "sqlite" },
});

const personal = {
    resources: { main: { declaration: database, retention: "retain", tags: {} } },
    installations: {
        notes: {
            package: {
                id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
                name: "@florian/notes",
                version: "2026.9.0",
            },
            alias: "notes",
            status: "enabled",
            resources: { [import.meta.destack.package.id]: { main: { resource: "main" } } },
            secrets: {},
            compute: {},
            tags: {},
        },
    },
} satisfies SpaceDefinition;

export const local = defineSpace(personal);
export const cloud = defineSpace({ ...personal });
```

## Policies

Declare package admission and outbound connections independently.

```ts
export const restricted = defineSpace({
    ...personal,
    policies: {
        packages: {
            admission: {
                default: "deny",
                rules: {
                    destack: { package: { kind: "destack" }, decision: "allow" },
                    npm: {
                        package: { kind: "npm", registry: "https://registry.npmjs.org/" },
                        decision: "allow",
                    },
                },
            },
        },
        network: {
            default: "deny",
            rules: {
                api: {
                    destination: {
                        kind: "hostname",
                        hostname: "api.example.com",
                        subdomains: false,
                    },
                    protocol: "https",
                    decision: "allow",
                },
            },
        },
    },
});
```

- Package rules cover the root package and every dependency, using their original registry identities.
- Deny rules take precedence within a policy; every applicable policy must permit the operation.
- Account and space policies apply together; installation and workload network policies add restrictions.
- Hostnames grant access to public addresses; private addresses require explicit CIDR grants.
- HTTPS and WSS default to port 443; HTTP and WS default to port 80.
- Raw TCP and UDP grants permit arbitrary protocols on the selected ports.
- Hosts check resolved addresses and redirects and reject policies they cannot enforce.
- Resource bindings authorise database and bucket access separately.

Omitted declarations leave independently managed policies unchanged.

## References

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

export const Parameters = defineSchema(schema.object({ alias: schema.string().min(1) }));

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

---
title: Package
description: The unit of everything.
---

# Package

- Destack is built around repositories and packages, for everything
- all libraries, services, applications, .. whatever live in packages, and packages live in repositories.
- "everything is an app" -> "everything is a package"

- packages contain applications, services, libraries, models, skills, templates, and assets
- think one or more npm-like package per .git monorepo
- everything is calver and uses a unified tagging / versioning scheme
- `template-*` packages contain versioned source for creating new packages
<!--- packages can be TS* or TS++-->

```text
Package                destack.json + package.json + source
├─ Declaration         Service, Connection, DB, Bucket, Vault, Secret,
│                      Setting, AuditAction, AccessObject, Schedule, Space
├─ Workload ─ start(context)
└─ View
```

- packages have `destack.json` similar to / next to `package.json`
- `package.json` holds name, version, dependencies and exports; `destack.json` holds the ID, targets, workloads, views and compute
- the package ID is immutable: renames keep it, forks get a new one

```json
{
    "id": "package-01996ab0-0000-7000-8000-000000000001",
    "language": "typescript",
    "targets": ["browser", "server"],
    "workloads": {
        "main": { "entrypoint": "./workload", "services": ["notes"], "schedules": ["reminders"] }
    },
    "views": {
        "editor": { "entrypoint": "./app" }
    }
}
```

- builds stamp ID, name and version into `import.meta.destack.package`
- declarations are inert: importing one never provisions, connects or grants
- declaration identity survives re-exports; bindings use it

- importing a shared package does not require binding every resource it exports
- public entrypoints separate declarations, clients and server implementations

## Declarations

- `declare/` owns definitions and declarations; `inspect/` owns descriptions; references live beside `Declaration` in `@destack/package`

```text
XDefinition ──defineX()──► X ──describeX()──► XDescription
(authored)                 │   (manifest/<domain>.json)
                           │
                           └──reference()──► DeclarationReference { packageId, name }
                                             (bindings, permissions, events, assignments)

X is a Declaration: an inert value with package and name, stamped by the build
```

## Layout

- `service/` provides APIs, `connection/` declares consumed APIs, and `server/` implements procedures.
- `stack/db.ts` declares database tables and schema histories; `stack/bucket.ts` and `stack/vault.ts` declare their resources
- `app/` contains client startup and composition; domain modules use package nouns such as db, bucket and vault.

```ts
// src/package.ts, exported as "@florian/notes/package"
import { definePackage } from "@destack/package/declare";
import { main } from "./stack/db.ts";
import { files } from "./stack/bucket.ts";

export default definePackage({ resources: { main, files } });
```

```text
@florian/notes/
├─ destack.json            id, targets, workloads, views
├─ package.json            name, version, dependencies, exports
├─ src/
│  ├─ package.ts           export default definePackage({ resources, secrets })
│  ├─ index.ts             re-exports only
│  ├─ stack/               db.ts · bucket.ts · vault.ts: resource and secret declarations
│  ├─ service/             defineService(...): procedures, client-safe
│  ├─ connection/          defineServiceConnection(...): consumed services, client-safe
│  ├─ server/              implementService(...): server-only
│  ├─ workload/            start(context)
│  ├─ settings/ audit/ access/   defineSetting · defineAuditAction · defineObject
│  ├─ app/                 view startup and composition
│  └─ <noun>/              domain modules, one noun each
└─ tests/                  *.test.ts
```

# Destack packages

These rules apply to every package with a `destack.json`, in any language.
The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY follow RFC 2119.
Every rule has a stable code, `PK` and a number.
New rules take the next free number, and retired codes stay unused.
A lint name in parentheses, like (`destack/valid-declaration`), marks a rule the linter enforces.

## Layout

```text
package/
├─ destack.json            id, language, template, targets, runtimes, exports, views
├─ package.json            name, version, dependencies, exports
├─ src/
│  ├─ package.ts           export default definePackage({ resources, secrets })
│  ├─ index.ts             re-exports only
│  ├─ stack/               db.ts · bucket.ts · vault.ts: resource and secret declarations
│  ├─ service/             defineService(...): procedures, client-safe
│  ├─ connection/          defineServiceConnection(...): consumed services, client-safe
│  ├─ server/              implementService(...): server-only
│  ├─ workload/            defineWorkload({ name, compute, start })
│  ├─ settings/ audit/ access/   defineSetting · defineAuditAction · defineObject
│  ├─ app/                 view startup and composition
│  └─ <noun>/              domain modules, one noun each
└─ tests/                  *.test.ts
```

- **PK01** Public service procedures MUST live in `service/`, and their implementation in `server/`.
- **PK02** Consumed service declarations MUST live in `connection/`, with `index.ts` re-exports.
- **PK03** Index modules MUST contain only re-exports. (`destack/no-index-logic`)
- **PK04** Reusable domain operations SHOULD live beside `server/` and take verified context explicitly.

## Declarations

```text
Definition     what the author passes      { name: "web", compute, start }      code, any values
   │ defineWorkload(definition)
Declaration    the stamped runtime value    { package, name, compute, start }    code, with behavior
   │ describeWorkload(declaration)          (build)
Description    JSON in the manifest         { name, compute }                    data
Reference      a pointer to a declaration   { packageId, name }                  data, identity only
```

- **PK05** Declarations MUST be exported module-level constants initialised by their `define*` constructor. (`destack/valid-declaration`)
- **PK06** Package handles MUST be default-exported from `src/package.ts`. (`destack/valid-package-handle`)
- **PK07** Package identity MUST come from `import.meta.destack` or package handles, never from imported `destack.json` or `package.json`. (`destack/no-manifest-import`)
- **PK08** `destack.json` MUST hold only what code cannot declare: identity, language, template, compatibility and views.

## Services

- **PK09** Connection declarations MUST stay inert; runtime clients bind during application startup or host invocation setup.
- **PK10** Endpoint discovery and credential renewal MUST live in host or client transport code, outside connection declarations.
- **PK11** `server/index.ts` MUST export `implementService(...)`, returning a `ServiceImplementation` that references its `Service` declaration.
- **PK12** Packages MUST NOT connect to their own services; they call their own domain code directly.

## Workloads

- **PK13** Workloads MUST be declared with `defineWorkload`; hosts MUST run them through `@destack/service`'s `WorkloadInstance`.
- **PK14** Workloads MUST release acquired resources through `context.defer` in `start`.

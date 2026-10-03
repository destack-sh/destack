---
title: Libraries
description: Destack first party packages.
---

# Libraries

Every Destack library is a package with a `destack.json` beside its `package.json`.
Its entry points separate shared code, declarations, descriptions and server implementations.

| Entry point | Holds                                                                   |
| ----------- | ----------------------------------------------------------------------- |
| `.`         | the package's shared nouns, schemas and clients                         |
| `declare`   | the `defineX` constructors, inert until a stack binds what they declare |
| `inspect`   | the `describeX` functions that write the package's built manifest       |
| `server`    | the service and object implementations a host serves                    |

## Packages

| Layer    | Package                                                                   | Purpose                                                                 |
| -------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Accounts | `@destack/account`                                                        | users, accounts, sign-in and the global directory                       |
|          | `@destack/access`                                                         | relate subjects to objects and decide their permissions                 |
|          | `@destack/audit`                                                          | declare audit actions, journal every executed call and query history    |
| Apps     | `@destack/view`, `@destack/signals`, `@destack/style`, `@destack/theme`   | render interfaces with Solid 2, StyleX and shared theme tokens          |
| Services | `@destack/service`                                                        | define, host and call HTTP services, their triggers and background work |
|          | `@destack/object`                                                         | declare object types with fields, traits, permissions and methods       |
|          | `@destack/notification`, `@destack/mail`                                  | deliver notifications to the inbox, desktop, push and email             |
|          | `@destack/setting`, `@destack/social`                                     | resolve scoped settings, and attach comments and reactions to objects   |
| Data     | `@destack/db`, `@destack/sync`                                            | declare, query and migrate tables, and keep query results current       |
|          | `@destack/bucket`, `@destack/vault`                                       | store files and versioned secrets                                       |
|          | `@destack/schema`                                                         | define, validate and describe data                                      |
| Source   | `@destack/package`, `@destack/resource`                                   | define packages, transform their modules and bind their resources       |
|          | `@destack/repository`                                                     | keep Git repositories and observe their branches and tags               |
| Hosts    | `@destack/host`, `@destack/space`, `@destack/relay`, `@destack/directory` | register hosts, serve spaces, and route names to the cells serving them |
|          | `@destack/telemetry`, `@destack/monitor`                                  | record and search logs, traces and metrics                              |
|          | `@destack/update`, `@destack/fs`, `@destack/test`                         | update distributions, use the host filesystem, and write tests          |

Each package's README documents its declarations and calls.

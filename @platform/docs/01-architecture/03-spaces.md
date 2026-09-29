---
title: Spaces
description: Spaces.
---

# Spaces

- "space" is the root organisational unit, owned by some account
- think "work" and "personal" or "team X" or such

- all apps in a single space use the same durability and and such database
- i.e. all packages in a space share trust, permissions, and a model.

- a space is where we install packages into
- holds all the durability for that space
- apps within a space can see each other directly

- spaces persist across restarts and changes of host
- @<user>/model packages centrally define schemas, relationships, and migrations
- applications and services use the shared model per space (?)

- packages can run multiple times with different configuration and records

- spaces communicate only through APIs (defined with library/service)

- human in the loop notifications and approvals

- "system space"
- protected system space stores host administration and grants.

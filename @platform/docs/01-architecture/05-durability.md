---
title: Durability
description: Durability.
---

# Durability

- each space has one "database" (conceptually)
- sqlite can scale very far for personal software
- single tenant/space DB with records, migrations, schedules, jobs, and execution results, ...
- s3-equivalent for files and large objects, maybe just.. folders locally

- Handoffs pause writes, transfer committed changes, revoke access to the old database, and activate the new one.
- migrations run at the authoritative database and record the resulting model version.

- execution state persists
- jobs persist inputs, completed steps, waits, retries, and cancellation state
- mini temporal? not sure to what degree we even need this. 
- but probably for agent style execution we do need this

- backups include database state, package versions, and referenced files.
- restoration is verified independently of the running space.
- export, import, and recovery remain available

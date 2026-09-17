---
title: Policy
description: Required host actions and resources.
---

# Policy

- packages declare required host actions and resources in `policy.requires`
- applications and workspaces decide access with ordered `policy.rules`
- each rule matches a subject such as a package, an action such as `fs.read` or `net.connect`, and a resource pattern
- bindings are the runtime enforcement point because every host interaction crosses a typed `@binding`

```json
{
    "policy": {
        "requires": [
            { "action": "fs.read", "resource": "app://config/**" },
            { "action": "net.connect", "resource": "tcp://database.internal:5432" }
        ],
        "rules": [
            {
                "subject": { "package": "@vendor/parser" },
                "action": "net.connect",
                "resource": "*",
                "access": "deny"
            }
        ]
    }
}
```

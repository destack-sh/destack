---
title: Policy
description: Runtime policy and controlled interaction with the host.
order: 32
---

# Policy

Destack supports configuring the policy that controls which host actions some piece of code - like a module or some dependency - may perform.
Packages declare the actions and resources they require, and the app or workspace decides which requirements are allowed.

```json:destack.json
{
    "policy": {
        "requires": [
            { "action": "fs.read", "resource": "app://config/**" },
            { "action": "net.connect", "resource": "tcp://database.internal:5432" }
        ],
        "rules": [
            {
                "subject": { "package": "app" },
                "action": "fs.read",
                "resource": "app://config/**",
                "access": "allow"
            },
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

---
title: Targets
description: Targets
---

# Targets

```json
{
  "$schema": "https://destack.sh/schemas/destack.schema.json",
  "name": "my-destack-project",
  "version": "0.1.2",
  "private": true,
  "targets": {
    "default": {
      "include": ["src/**/*.ds"],
      "output": "program"
    }
  },
  "defaultTarget": "default"
}
```

# Conditions

Conditions select sources per target at compile time.

## source selection

### active condition files share the base module

Named modes, roles, features, and tags are automatic file aliases.

```ds:user.ds
export const base = "user";
```

```ds:user.preview.ds
export const previewName = `${base}.preview`;
```

```ds:user.server.ds
export const serverName = `${base}.server`;
```

```ds:user.checkout.ds
export const checkoutName = `${base}.checkout`;
```

```ds:user.internal.ds
export const internalName = `${base}.internal`;
```

```ds:main.ds
import { base, checkoutName, internalName, previewName, serverName } from "./user";

base satisfies "user";
previewName satisfies "user.preview";
serverName satisfies "user.server";
checkoutName satisfies "user.checkout";
internalName satisfies "user.internal";
```

```json:destack.json
{
    "conditions": {
        "modes": {
            "preview": { "extends": "dev" }
        },
        "features": {
            "checkout": {}
        },
        "tags": {
            "internal": {}
        }
    },
    "compiler": {
        "modes": ["preview"],
        "features": ["checkout"],
        "tags": ["internal"]
    },
    "targets": {
        "default": {
            "emit": "js",
            "roles": ["server"]
        }
    },
    "defaultTarget": "default"
}
```

### explicit aliases can target release stage

Stage aliases match the configured release stage.

```ds:user.ds
export const base = "user";
```

```ds:user.alpha.ds
export const alphaName = `${base}.alpha`;
```

```ds:main.ds
import { alphaName, base } from "./user";

base satisfies "user";
alphaName satisfies "user.alpha";
```

```json:destack.json
{
    "version": "2026.5.27-alpha.1",
    "conditions": {
        "aliases": {
            "alpha": { "stage": "alpha" }
        }
    },
    "products": {
        "cli": {
            "stage": "alpha",
            "targets": {
                "main": "default"
            }
        }
    },
    "targets": {
        "default": {
            "emit": "js"
        }
    },
    "defaultProduct": "cli",
    "defaultTarget": "default"
}
```

### inactive condition files are ignored

Conditional files only contribute when their alias gate matches.

```ds:user.ds
export const base = "user";
```

```ds:user.checkout.ds
export const checkoutName = `${base}.checkout`;
```

```ds:main.ds
import { base } from "./user";

base satisfies "user";
```

```json:destack.json
{
    "conditions": {
        "features": {
            "checkout": {}
        }
    },
    "targets": {
        "default": {
            "emit": "js"
        }
    },
    "defaultTarget": "default"
}
```

### explicit aliases can target profile gates

Explicit aliases can refer to scalar target metadata such as host.

```ds:user.ds
export const base = "user";
```

```ds:user.browser.ds
export const browserName = `${base}.browser`;
```

```ds:main.ds
import { base, browserName } from "./user";

base satisfies "user";
browserName satisfies "user.browser";
```

```json:destack.json
{
    "conditions": {
        "aliases": {
            "browser": { "host": "browser" }
        }
    },
    "targets": {
        "default": {
            "emit": "js",
            "host": "browser"
        }
    },
    "defaultTarget": "default"
}
```

### chained aliases require every gate

Chained condition files only contribute when every alias matches.

```ds:user.ds
export const base = "user";
```

```ds:user.test.browser.ds
export const browserTestName = `${base}.test.browser`;
```

```ds:main.ds
import { base, browserTestName } from "./user";

base satisfies "user";
browserTestName satisfies "user.test.browser";
```

```json:destack.json
{
    "conditions": {
        "aliases": {
            "browser": { "host": "browser" }
        }
    },
    "compiler": {
        "modes": ["test"]
    },
    "targets": {
        "default": {
            "emit": "js",
            "host": "browser"
        }
    },
    "defaultTarget": "default"
}
```

# workspace

`destack.json` is the authoritative project manifest unifying project metadata, toolchain settings, build targets, and deployment topology in one schema.
We still read `tsconfig.json` and `package.json` where needed for compatibility, but the long term direction is to abosrb everything into `destack.json.

```text
Session
    │
    ├── Workspace
    │       │
    │       └── Program
    │               │
    │               ├── Package
    │               │       └── Module
    │               │
    │               ├── ArtifactRegistry
    │               └── OutputRegistry
```

## Example

```jsonc
{
    "name": "@destack/app",
    "private": true,
    "workspace": {
        "members": ["apps/*", "packages/*"]
    },
    "tasks": {
        "dev": "destack dev",
        "build": "destack build"
    },
    "compiler": {
        "strict": true
    },
    "runtime": {
        "execution": "record"
    },
    "targets": {
        "web": {
            "output": "js",
            "runtime": "browser",
            "platform": "browser"
        },
        "api": {
            "output": "js",
            "runtime": "node"
        }
    },
    "accounts": {
        "cloudflareMain": {
            "provider": "cloudflare",
            "mode": "secret"
        }
    },
    "stacks": {
        "production": {
            "components": {
                "client": {
                    "type": "destack.sh/component/client",
                    "with": {
                        "target": "web",
                        "domain": "app.example.com"
                    }
                },
                "api": {
                    "type": "destack.sh/component/service",
                    "with": {
                        "target": "api",
                        "type": "destack.sh/service/http",
                        "protocol": "http",
                        "mount": "/api"
                    }
                }
            }
        }
    }
}
```

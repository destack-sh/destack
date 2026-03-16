# workspace

The workspace crate owns most of the shared Destack language toolchain state.
We don't define (much) logic here, it's mostly about defining the containers and registries that make compiler, daemon, tests, and language tools work.

## Configuration

The `config/` module defines unified toolchain configuration.
That includes compiler, runtime, formatter, linter, cache, daemon, and target options.
Target configuration is also where app declarations belong.
Those declarations describe what the app intends to use from the surrounding platform, and later lower into platform-native manifests and packaging metadata.

### destack.json

`destack.json` is the authoritative project manifest unifying project metadata, toolchain settings, build targets, and deployment topology in one schema.
We still read `tsconfig.json` and `package.json` where needed for compatibility, but the long term direction is to absorb everything into `destack.json`.

```json
{
    "compiler": {
        "target": "es2024",
        "strict": true,
        "noImplicitAny": true,
        "incremental": true
    },
    "runtime": {
        "execution": "record",
        "time": { "mode": "virtual" },
        "random": { "mode": "deterministic", "seed": 1337 }
    },
    "cache": {
        "mode": "disk",
        "dir": ".destack/cache",
        "maxSizeMb": 2048,
        "policy": "lru",
        "validate": "strict"
    },
    "watch": {
        "debounceMs": 30
    },
    "daemon": {
        "idleShutdownMs": 600000
    },
    "formatter": {
        "lineWidth": 100,
        "indentStyle": "space"
    },
    "linter": {
        "preset": "recommended"
    },
    "targets": {
        "web": { "output": "js", "platform": "browser" },
        "iosApp": {
            "output": "native",
            "platform": "ios",
            "app": {
                "permissions": {
                    "camera": { "usage": "Scan receipts" },
                    "microphone": { "usage": "Record voice notes" },
                    "location": { "usage": "Show nearby pickup points" },
                    "locationBackground": { "usage": "Track active delivery trips" }
                },
                "intents": {
                    "querySchemes": ["https", "mailto"],
                    "sharesFiles": true,
                    "handledSchemes": ["destack-demo"],
                    "verifiedDomains": ["app.example.com"]
                },
                "notifications": {
                    "enabled": true,
                    "remote": true,
                    "badges": true,
                    "sounds": true,
                    "timeSensitive": true
                },
                "background": {
                    "modes": ["audio"]
                },
                "location": {
                    "allowsBackgroundUpdates": true,
                    "preciseByDefault": true,
                    "temporaryPrecisePurposes": ["turnByTurnNavigation"]
                },
                "document": {
                    "openTypes": ["public.image"],
                    "saveTypes": ["public.plain-text"],
                    "supportsOpenInPlace": true
                },
                "credentials": {
                    "biometricUsage": "Unlock saved credentials",
                    "accessGroups": ["group.com.example.shared"],
                    "credentialDomains": ["app.example.com"]
                }
            }
        },
        "macosApp": {
            "output": "native",
            "platform": "macos",
            "app": {
                "intents": {
                    "querySchemes": ["https", "mailto"],
                    "verifiedDomains": ["app.example.com"],
                    "handledFileTypes": ["public.image", ".png"]
                },
                "notifications": {
                    "enabled": true,
                    "badges": true,
                    "sounds": true,
                    "categories": [
                        {
                            "identifier": "messages",
                            "actions": [
                                {
                                    "identifier": "reply",
                                    "title": "Reply",
                                    "style": "textInput",
                                    "foreground": true,
                                    "textInputButtonTitle": "Send",
                                    "textInputPlaceholder": "Message"
                                }
                            ]
                        }
                    ]
                },
                "services": {
                    "foregroundModes": ["dataSync"]
                },
                "document": {
                    "openTypes": ["public.image"],
                    "saveTypes": ["public.plain-text"],
                    "supportsOpenInPlace": true
                }
            }
        },
        "androidApp": {
            "output": "native",
            "platform": "android",
            "app": {
                "permissions": {
                    "camera": { "usage": "Capture one receipt image" },
                    "notifications": { "usage": "Send delivery alerts" }
                },
                "intents": {
                    "querySchemes": ["geo", "mailto"],
                    "sharesFiles": true,
                    "handledSchemes": ["destack-demo"],
                    "verifiedDomains": ["app.example.com"],
                    "handledShareTypes": ["image/*"]
                },
                "notifications": {
                    "enabled": true,
                    "remote": true
                },
                "background": {
                    "modes": ["remoteNotifications", "location"]
                },
                "services": {
                    "foregroundModes": ["location", "dataSync"]
                },
                "document": {
                    "openTypes": ["image/*"],
                    "saveTypes": ["text/plain"]
                }
            }
        }
    }
}
```

Child packages inherit from parent `destack.json` with restrictive merge semantics where appropriate.
The `app` section is target-scoped rather than global.
That keeps app declarations beside the concrete build target that lowers into Android manifests, Apple property lists and entitlements, Windows manifests, or other host packaging metadata.
It is intentionally semantic rather than syntactic: the config describes what the app declares, and platform-native manifest formats are lowering targets rather than the source language.

## Containers

The containment model is intentionally simple.

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

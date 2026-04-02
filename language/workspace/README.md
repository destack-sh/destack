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
        "mode": "disk"
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
        "web": { "emit": "js", "platform": "browser" },
        "iosApp": {
            "emit": "native",
            "platform": "ios",
            "app": {
                "identity": {
                    "identifier": "com.example.destackdemo",
                    "displayName": "Destack Demo",
                    "iconPath": "assets/icon.png"
                },
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
            "emit": "native",
            "platform": "macos",
            "app": {
                "identity": {
                    "identifier": "com.example.destackdemo",
                    "displayName": "Destack Demo",
                    "iconPath": "assets/icon.png"
                },
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
            "emit": "native",
            "platform": "android",
            "app": {
                "identity": {
                    "identifier": "com.example.destackdemo",
                    "displayName": "Destack Demo",
                    "iconPath": "assets/icon.png"
                },
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

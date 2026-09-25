# Object Comments

## Comments Causing Expansion

### comment in object stays inline when short

Short objects with internal comments stay inline.

```tspp:main.tspp
({ /* key */ a: 1, /* another */ b: 2 })
```

The formatter keeps this object inline when it fits.

```tspp expected
({ /* key */ a: 1, /* another */ b: 2 });
```

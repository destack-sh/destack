# Object Comments

## Comments Causing Expansion

### comment in object stays inline when short

Short objects with internal comments stay inline.

```ds:main.ds
({ /* key */ a: 1, /* another */ b: 2 })
```

The formatter keeps this object inline when it fits.

```ds expected
({ /* key */ a: 1, /* another */ b: 2 });
```

### comment in computed object key

Comments before computed keys stay attached to the key.

```ds:main.ds
({ /* key */ [k]: value })
```

```ds expected
({ /* key */ [k]: value });
```

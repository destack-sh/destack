# Object Comments

## Comments Causing Expansion

### comment in object stays inline when short

Short objects with internal comments stay inline.

```ts:main.ts
({ /* key */ a: 1, /* another */ b: 2 })
```

The formatter keeps this object inline when it fits.

```ts expected
({ /* key */ a: 1, /* another */ b: 2 });
```

### comment in computed object key

Comments before computed keys stay attached to the key.

```ts:main.ts
({ /* key */ [k]: value })
```

```ts expected
({ /* key */ [k]: value });
```

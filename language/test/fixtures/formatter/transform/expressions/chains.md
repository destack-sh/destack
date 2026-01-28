# Member and Call Chains

Tests for chained member access and call formatting.

## Head Group

### chain keeps short head group

Short head groups stay on the first line when breaking chains.

```ds line-width=30
const result = api.getClient().getService().fetchAll().map((x) => x.id)
```

```ds expected
const result = api.getClient()
    .getService()
    .fetchAll()
    .map((x) => x.id);
```

## Optional Chaining

### optional chain breaks with leading operators

Optional chains break with the `?.` operator leading each line.

```ds line-width=35
const value = dataSource?.getClient()?.getUser(id)?.profile?.name
```

```ds expected
const value = dataSource
    ?.getClient()
    ?.getUser(id)
    ?.profile
    ?.name;
```

### optional chain with computed access

Optional chaining keeps computed access tight.

```ds
const value = api?.users?.[0]?.profile?.["full-name"]
```

```ds expected
const value = api?.users?.[0]?.profile?.["full-name"];
```

## Non-Null Assertions

### non-null assertions stay tight

Non-null assertions keep tight spacing.

```ds
const value = maybe!.nested!.value
```

```ds expected
const value = maybe!.nested!.value;
```

## Computed Access

### computed access stays inline

Computed member access stays inline when short.

```ds
const value = client.users[0]["full-name"].toString()
```

```ds expected
const value = client.users[0]["full-name"].toString();
```

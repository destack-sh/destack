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

### chain with long generic call arguments

Long generic calls break their argument list and keep the chain head intact.

```ts:main.ts line-width=60
const defaultColorDecoratorsEnablement = accessor.get(IConfigurationService).getValue<"auto" | "always" | "never">("longlonglonglonglonglonglonglonglong")
```

```ts expected
const defaultColorDecoratorsEnablement = accessor
    .get(IConfigurationService)
    .getValue<"auto" | "always" | "never">(
        "longlonglonglonglonglonglonglonglong",
    );
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

### optional chain with trailing comments

Trailing comments stay attached to the chain segment they follow.

```ds line-width=60
this.getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
  ?.();

this
  .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
  ?.();

foo
  .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
  ?.();

getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
  ?.();
```

```ds expected
this.getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();

this
    .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();

foo
    .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();

getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();
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

## Commented Chains

### chain with inline comments

Inline comments stay attached to their chain segment.

```ds line-width=80
wow /* do something weird here */
  .omg! /* do something weird here */
  .map((x) => x.name) /* do something weird here */
  .filter((x) => x.length > 3)
  .sort((a, b) => a.length - b.length)
```

```ds expected
wow /* do something weird here */
    .omg! /* do something weird here */
    .map((x) => x.name) /* do something weird here */
    .filter((x) => x.length > 3)
    .sort((a, b) => a.length - b.length);
```

### chain preserves blank lines

Blank lines between chain segments are preserved.

```ts:main.ts
Promise.all(writeIconFiles)
  // TO DO -- END
  .then(() => writeRegistry())

Promise.all(writeIconFiles)

  // TO DO -- END
  .then(() => writeRegistry())

Promise.all(writeIconFiles)
  // TO DO -- END

  .then(() => writeRegistry())
```

```ts expected
Promise.all(writeIconFiles)
    // TO DO -- END
    .then(() => writeRegistry());

Promise.all(writeIconFiles)

    // TO DO -- END
    .then(() => writeRegistry());

Promise.all(writeIconFiles)
    // TO DO -- END

    .then(() => writeRegistry());
```

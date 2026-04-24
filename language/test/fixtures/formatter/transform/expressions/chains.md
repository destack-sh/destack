# Member and Call Chains

Tests for chained member access and call formatting.

## Head Group

### chain keeps short head group

Short head groups stay on the first line when breaking chains.

```ds line-width=30
const result = api.getClient().getService().fetchAll().map((x) => x.id)
```

```ds expected
const result = api
    .getClient()
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
    ?.getUser(id)?.profile?.name;
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
this
    .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();

this
    .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();

foo
    .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */
    ?.();

getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */?.();
```

## Instantiation Expressions

### member instantiation stays inline

Instantiation expressions stay attached to the member expression they instantiate.

```ts:main.ts
const factory = api.getFactory<number>
```

```ts expected
const factory = api.getFactory<number>;
```

### instantiation after computed access stays inline

Instantiation expressions after computed access keep type arguments attached.

```ts:main.ts
const factory = providers["main"]<Factory>
```

```ts expected
const factory = providers["main"]<Factory>;
```

### member instantiation stays inline at fixture width

Short member instantiation chains stay inline at this fixture width while keeping the type arguments attached.

```ts:main.ts line-width=25
const value = api.getService().getFactory<number>
```

```ts expected
const value = api.getService().getFactory<number>;
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

### static members after non-null assertions stay tight

Non-null assertions inside static-member chains keep the following supported member path intact.

```ts:main.ts
compoundConfigurationsSchema.items.oneOf![1].properties!.folder.enum = folderNames
```

```ts expected
compoundConfigurationsSchema.items.oneOf![1].properties!.folder.enum = folderNames;
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

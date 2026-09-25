# Member and Call Chains

Chain fixtures cover member access, calls, optional chains, instantiation, and chain comments.

## Head Group

### chain keeps short head group

Short head groups stay on the first line when breaking chains.

```tspp line-width=30
const result = api.getClient().getService().fetchAll().map((x) => x.id)
```

```tspp expected
const result = api
    .getClient()
    .getService()
    .fetchAll()
    .map((x) => x.id);
```

### chain with long generic call arguments

Long generic calls break their argument list and keep the chain head intact.

```tspp:main.tspp line-width=60
const defaultColorDecoratorsEnablement = accessor.get(IConfigurationService).getValue<"auto" | "always" | "never">("longlonglonglonglonglonglonglonglong")
```

```tspp expected
const defaultColorDecoratorsEnablement = accessor
    .get(IConfigurationService)
    .getValue<"auto" | "always" | "never">(
        "longlonglonglonglonglonglonglonglong",
    );
```

## Optional Chaining

### optional chain breaks with leading operators

Optional chains break with the `?.` operator leading each line.

```tspp line-width=35
const value = dataSource?.getClient()?.getUser(id)?.profile?.name
```

```tspp expected
const value = dataSource
    ?.getClient()
    ?.getUser(id)?.profile?.name;
```

### optional chain with computed access

Optional chaining keeps computed access tight.

```tspp
const value = api?.users?.[0]?.profile?.["full-name"]
```

```tspp expected
const value = api?.users?.[0]?.profile?.["full-name"];
```

### optional chaining cascade

Multiple optional chaining operators stay compact when they fit.

```tspp
const value = obj?.nested?.deeply?.value
```

```tspp expected
const value = obj?.nested?.deeply?.value;
```

### optional chain with trailing comments

Trailing comments stay attached to the chain segment they follow.

```tspp line-width=60
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

```tspp expected
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

```tspp:main.tspp
const factory = api.getFactory<number>
```

```tspp expected
const factory = api.getFactory<number>;
```

### instantiation after computed access stays inline

Instantiation expressions after computed access keep type arguments attached.

```tspp:main.tspp
const factory = providers["main"]<Factory>
```

```tspp expected
const factory = providers["main"]<Factory>;
```

### member instantiation stays inline at fixture width

Member instantiation chains break at this fixture width while keeping the type arguments attached.

```tspp:main.tspp line-width=25
const value = api.getService().getFactory<number>
```

```tspp expected
const value =
    api.getService()
        .getFactory<number>;
```

## Non-Null Assertions

### non-null assertions stay tight

Non-null assertions keep tight spacing.

```tspp
const value = maybe!.nested!.value
```

```tspp expected
const value = maybe!.nested!.value;
```

### static members after non-null assertions stay tight

Non-null assertions inside static-member chains keep the following member path intact.

```tspp:main.tspp
compoundConfigurationsSchema.items.oneOf![1].properties!.folder.enum = folderNames
```

```tspp expected
compoundConfigurationsSchema.items.oneOf![1].properties!.folder.enum = folderNames;
```

## Computed Access

### computed access stays inline

Computed member access stays inline when short.

```tspp
const value = client.users[0]["full-name"].toString()
```

```tspp expected
const value = client.users[0]["full-name"].toString();
```

## Commented Chains

### chain with inline comments

Inline comments stay attached to their chain segment.

```tspp line-width=80
wow /* do something weird here */
  .omg! /* do something weird here */
  .map((x) => x.name) /* do something weird here */
  .filter((x) => x.length > 3)
  .sort((a, b) => a.length - b.length)
```

```tspp expected
wow /* do something weird here */
    .omg! /* do something weird here */
    .map((x) => x.name) /* do something weird here */
    .filter((x) => x.length > 3)
    .sort((a, b) => a.length - b.length);
```

### chain preserves blank lines

Blank lines between chain segments are preserved.

```tspp:main.tspp
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

```tspp expected
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

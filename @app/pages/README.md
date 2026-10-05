# @app/pages

Write pages in nested trees, shared with people or published by link.

## Objects

`page` is the package's object type, and a subpage inherits the grants of the pages above it.

```ts
const root = await client.page.create({ spaceId, requestId, title: "Handbook" });
const child = await client.page.create({
    spaceId,
    requestId,
    parentId: root.id,
    title: "Onboarding",
});
```

### Order

`position` orders a page among its siblings as a fractional index.

```ts
await client.page.update({ spaceId, id: child.id, requestId, position: "a1" });
```

### Trash

`page.delete` moves a page to the trash for 30 days, and `page.restore` brings it back for a person who manages it.

```ts
await client.page.delete({ spaceId, id: child.id, requestId });
await client.page.restore({ spaceId, id: child.id, requestId });
```

### Links

`page.grant` with the `viewer` relation, the `anyone` subject and a link secret condition publishes a page and its subpages to everyone with the link.

```ts
import { anyone, LinkSecret } from "@destack/access";

const link = await LinkSecret.create();
await client.page.grant({
    spaceId,
    id: root.id,
    requestId,
    relation: "viewer",
    subject: anyone.reference("*", "*"),
    conditions: { linkSecret: link.digest },
});
```

## Service

`pagesService` serves pages with their trees, sharing, links and trash.

```ts
import { pagesService } from "@app/pages/service";
```

## Tables

`pagesTables` adds the page tables to another package's database, and `install(pages, …)` installs the package with its `main` database.

```ts
import pages from "@app/pages/package";

export const personal = defineSpace({ installations: { pages: install(pages, { main: "main" }) } });
```

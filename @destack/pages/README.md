Write pages in nested trees, shared with people or published by link.

## Pages

Pages use the default shareable roles, and every page below a page inherits its sharing.

```ts
const root = await client.page.create({ spaceId, requestId, title: "Handbook" });
const child = await client.page.create({
    spaceId,
    requestId,
    parentId: root.id,
    title: "Onboarding",
});
```

A page keeps its place among its siblings in `position`, and a deleted page stays in the trash for 30 days.

## Links

A `viewer` grant to anyone presenting a link's secret publishes the page and its subpages.

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

## Installation

A stack installs the package with the database its pages live in, and another package embeds `pagesTables` in a database of its own.

```ts
import pages from "@destack/pages/package";

export const personal = defineSpace({ installations: { pages: install(pages, { main: "main" }) } });
```

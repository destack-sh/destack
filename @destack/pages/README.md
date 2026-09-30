Write pages in nested trees, shared and linked across spaces.

A page shares with its subpages: whoever reads, edits or manages a page does the same on every page below it.

| Relation | `read` | `edit` | `manage` |
|---|---|---|---|
| `owner` | yes | yes | yes |
| `editor` | yes | yes | |
| `viewer` | yes | | |

A `viewer` may be anyone presenting a link's secret, which publishes the page and its subpages, and a deleted page stays in the trash for 30 days.

```ts
const root = await client.page.create({ spaceId, requestId, title: "Handbook" });
const child = await client.page.create({ spaceId, requestId, parentId: root.id, title: "Onboarding" });
const link = await Capability.create();
await client.page.grant({
    spaceId,
    id: root.id,
    requestId,
    relation: "viewer",
    subject: anyone.reference("*", "*"),
    conditions: { capability: link.digest },
});
```

A stack installs the package with the database its pages live in, and another package embeds `pagesTables` in a database of its own.

```ts
import pages from "@destack/pages/package";

export const personal = defineSpace({ installations: { pages: install(pages, { main: "main" }) } });
```

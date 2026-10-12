import type { Catalog } from "@destack/locale";
import { defineDatabase, type Dialect } from "@destack/db";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { activity, announcement, subscription } from "../../src/index.ts";
import { serveActivities } from "../../src/server/index.ts";
import { NotificationFixture } from "../../src/test/index.ts";
import { actors } from "./actor.ts";
import { changed, document, notes, notifications, summarizeChanges } from "./document.ts";

export { type Actor, actors } from "./actor.ts";
export { documentParent } from "./document.ts";

/** The start of every scenario: Monday 28 September 2026, 10:00 UTC, noon in Vienna. */
const MONDAY = Date.UTC(2026, 8, 28, 10, 0);

/** The documents' package's German catalog, which its build ships. */
const NOTES_CATALOG: Catalog = {
    package: notes.id,
    locale: "de",
    messages: {
        [changed.id]: "Dokument geändert",
        [summarizeChanges(1).id]:
            ".input {$p0 :number}\n.match $p0\none {{{$p0} Änderung}}\n* {{{$p0} Änderungen}}",
    },
};

/** The documents' package's build, shipping its German catalog. */
export const NOTES_BUILD: BuildReader = await MemoryBuild.write(
    new Map([["locale/de.json", new TextEncoder().encode(JSON.stringify(NOTES_CATALOG))]]),
    { package: notes },
);

/** The scenarios' database: the documents, their subscriptions, activities and announcements, beside the fixture's own tables. */
export const documentDatabase = defineDatabase({
    name: "main",
    tables: [
        ...NotificationFixture.tables,
        ...[document, subscription, activity, announcement].flatMap((object) => object.tables),
    ],
    copies: NotificationFixture.copies,
});

/** Serve documents and their activities in a new space as alice, on Monday unless given a start, batching announcements when given. */
export function serveSpace(
    dialect: Dialect,
    options: { readonly batch?: number; readonly at?: number } = {},
) {
    const { batch, at = MONDAY } = options;

    return NotificationFixture.open({
        dialect,
        origin: { package: document.package, service: "notification" },
        database: documentDatabase,
        objects: {
            document,
            ...serveActivities({ notifications, ...(batch === undefined ? {} : { batch }) }),
        },
        people: actors,
        actor: "alice",
        build: NOTES_BUILD,
        at,
        // read the documents its announcements expand from
        requested: [document.permission("read")],
    });
}

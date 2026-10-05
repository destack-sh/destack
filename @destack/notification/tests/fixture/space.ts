import type { Catalog } from "@destack/locale";
import { defineDatabase, type Dialect } from "@destack/db";
import { memoryBuild } from "@destack/host/test";
import type { BuildReader } from "@destack/package/manifest";
import { activity, announcement, subscription } from "../../src/index.ts";
import { serveNotifications } from "../../src/server/index.ts";
import { NotificationFixture } from "../../src/test/index.ts";
import { actors } from "./actor.ts";
import { changed, document, notes, notifications, summarizeChanges } from "./document.ts";

export { type Actor, actors } from "./actor.ts";

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
export const NOTES_BUILD: BuildReader = await memoryBuild(
    notes,
    {},
    new Map([["locale/de.json", new TextEncoder().encode(JSON.stringify(NOTES_CATALOG))]]),
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

/** Serve documents and their activities in a new space on Monday as alice, batching announcements when given. */
export function serveSpace(dialect: Dialect, options: { readonly batch?: number } = {}) {
    const { batch } = options;

    return NotificationFixture.open({
        dialect,
        origin: { package: document.package, service: "notification" },
        database: documentDatabase,
        objects: {
            document,
            subscription,
            ...serveNotifications({ notifications, ...(batch === undefined ? {} : { batch }) }),
        },
        people: actors,
        actor: "alice",
        build: NOTES_BUILD,
        at: MONDAY,
    });
}

/** Refer to a document as its attachments' parent. */
export function documentParent(id: string) {
    return {
        parent: { packageId: document.policy.definition.packageId, type: document.name, id },
    };
}

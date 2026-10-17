import type { Catalog } from "@destack/locale";
import { defineDatabase, type Dialect } from "@destack/db";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { account, client, key, space, user } from "@destack/account/object";
import { setting } from "@destack/setting/object";
import { notification } from "../../src/index.ts";
import { notificationTables } from "../../src/stack/db.ts";
import { NotificationFixture } from "../../src/test/index.ts";
import { actors } from "./actor.ts";
import { changed, changes, document, notes, notifications } from "./document.ts";

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
        [changes.id]:
            ".input {$count :number}\n.match $count\none {{{$count} Änderung}}\n* {{{$count} Änderungen}}",
    },
};

/** The documents' package's build, declaring its notifications and shipping its German catalog. */
export const NOTES_BUILD: BuildReader = await MemoryBuild.declaring(
    notes,
    notifications.map((declared) => ({
        kind: "notification",
        package: notification.package.id,
        name: declared.name,
        description: declared.describe(),
    })),
    {
        files: new Map([
            ["locale/de.json", new TextEncoder().encode(JSON.stringify(NOTES_CATALOG))],
        ]),
    },
);

/** The scenarios' database: the documents and their activities beside the fixture's tables. */
export const documentDatabase = defineDatabase({
    name: "main",
    tables: [
        ...NotificationFixture.tables,
        ...notificationTables,
        ...[document, setting].flatMap((object) => object.tables),
    ],
    replicated: [user.table, client.table, key.table, space.table, account.table],
});

/** Serve documents and their activities in a new space as alice, on Monday by default. */
export function serveSpace(
    dialect: Dialect,
    options: { readonly batch?: number; readonly at?: number } = {},
) {
    const { batch, at = MONDAY } = options;

    return NotificationFixture.open({
        dialect,
        origin: { package: document.package },
        database: documentDatabase,
        objects: [document],
        people: actors,
        actor: "alice",
        build: NOTES_BUILD,
        ...(batch === undefined ? {} : { batch }),
        at,
        // read the documents its announcements expand from
        requested: [document.permission("read")],
    });
}

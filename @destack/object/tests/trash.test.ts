import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { defineObject, definePackage, field } from "../src/index.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { space } from "./fixture/space.ts";
import { users } from "./fixture/user.ts";

/** A folder of pages, trashed and restored with them. */
const folder = defineObject(
    {
        name: "folder",
        plural: "folders",
        scope: space,
        fields: { name: field.string(schema.string().min(1)) },
        recoverable: { within: { days: 30 }, by: "manage", purge: "manage" },
        shareable: {},
    },
    (method) => ({ create: method.create("manage") }),
);

/** A page of a folder, going to the trash with its folder. */
const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    nested: { in: folder, delete: "trash", receive: "manage" },
    fields: { title: field.string(schema.string().min(1)) },
    recoverable: { within: { days: 30 }, by: "manage" },
    shareable: {},
});

/** Folders and their pages. */
const pagesPackage = definePackage({ objects: { folder, page } });

test("trash a folder's pages with it, restore only the ones trashed with it, refuse a page in a trashed folder, and purge it with its pages", async () => {
    // serve alice's folder with two pages, one trashed on its own first
    await using objects = await TestObjectStore.open({
        packages: [pagesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000300"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(pagesPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(Date.now()) });
    const kept = await client.folder.create({ ...request(), name: "Plans" });
    await client.page.create({ ...request(), parentId: kept.id, title: "First" });
    const second = await client.page.create({ ...request(), parentId: kept.id, title: "Second" });
    await client.page.delete({ ...request(), id: second.id });

    // trash the folder, then restore it
    await client.folder.delete({ ...request(), id: kept.id });
    const trashed = await objects.database.select().from(page.table);
    const [folderRow] = await objects.database.select().from(folder.table);
    const refused = await refusal(
        client.page.create({ ...request(), parentId: kept.id, title: "Third" }),
    );
    await client.folder.restore({ ...request(), id: kept.id });
    const restored = await objects.database.select().from(page.table);

    // purge the folder from the trash with its pages
    await client.folder.delete({ ...request(), id: kept.id });
    await client.folder.purge({ ...request(), id: kept.id });
    const purged = await objects.database.select().from(page.table);

    // trash the first page with the folder and restore it alone, the second staying trashed
    const folderDeleted = folderRow?.deletedAt;
    const secondDeleted = trashed.find((row) => row.id === second.id)?.deletedAt;
    expect({
        trashed: deletedAt(trashed),
        refused,
        restored: deletedAt(restored),
        purged,
    }).toEqual({
        trashed: [
            ["First", folderDeleted],
            ["Second", secondDeleted],
        ],
        refused: ["CONFLICT", "IN_TRASH", `folder ${kept.id} is in the trash`],
        restored: [
            ["First", "live"],
            ["Second", secondDeleted],
        ],
        purged: [],
    });
});

test("refuse a child trashed with its parent without a trash, or with a shorter window than its parent's", () => {
    // validate a note type under the folder with a given trash, as serving it does
    const declare = (recoverable: { within: { days: number }; by: "manage" } | undefined) => () =>
        defineObject({
            name: "note",
            plural: "notes",
            scope: space,
            nested: { in: folder, delete: "trash", receive: "manage" },
            fields: {},
            ...(recoverable === undefined ? {} : { recoverable }),
            shareable: {},
        }).requireValid();

    expect([
        refusalOf(declare(undefined)),
        refusalOf(declare({ within: { days: 7 }, by: "manage" })),
    ]).toEqual([
        "object note moves to the trash with its parent, so both must have a trash",
        "object note moves to the trash with its parent, so it must stay recoverable as long as its parent",
    ]);
});

/** Read the message a declaration throws. */
function refusalOf(declare: () => unknown): string {
    try {
        declare();

        return "declared";
    } catch (error) {
        return error instanceof TypeError ? error.message : "not a type error";
    }
}

/** List pages by title with their deletion time, or live. */
function deletedAt(rows: readonly { readonly title: string; readonly deletedAt: number | null }[]) {
    return rows
        .map((row) => [row.title, row.deletedAt === null ? "live" : row.deletedAt] as const)
        .toSorted((left, right) => left[0].localeCompare(right[0]));
}

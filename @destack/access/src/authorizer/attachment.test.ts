import { expect, onTestFinished, test } from "@destack/test";
import { asc, defineTable, text } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import {
    ACCESS_TABLES,
    Authorization,
    Authorizer,
    Policy,
    principal,
    relation,
    through,
    type AccessContext,
    type TableMapping,
} from "../index.ts";
import { module1 } from "../test/fixture.ts";

/** Remarks attached to any object whose type contributes itself to their parent. */
const remark = new Policy(module1.package, {
    name: "remark",
    relations: { parent: { subjects: [], open: true, grantedBy: null } },
    permissions: { read: through("parent", "read") },
});

/** Articles their owner reads, which take remarks. */
const article = new Policy(module1.package, {
    name: "article",
    relations: { owner: { subjects: [principal.user], grantedBy: null } },
    permissions: { read: relation("owner") },
    contributes: [{ policy: remark, relation: "parent" }],
});

/** Photos their owner reads, which take remarks too. */
const photo = new Policy(module1.package, {
    name: "photo",
    relations: { owner: { subjects: [principal.user], grantedBy: null } },
    permissions: { read: relation("owner") },
    contributes: [{ policy: remark, relation: "parent" }],
});

/** The rows of an owned host type. */
function hostTable(name: string) {
    return defineTable(name, {
        id: text("id").primaryKey().notNull(),
        scope: text("scope").notNull(),
        owner: text("owner").notNull(),
    });
}

/** The article rows. */
const articles = hostTable("attachment_article");
/** The photo rows. */
const photos = hostTable("attachment_photo");
/** The remark rows, each naming its parent's package, type and identifier. */
const remarks = defineTable("attachment_remark", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
    parentPackageId: text("parent_package_id").notNull(),
    parentType: text("parent_type").notNull(),
    parentId: text("parent_id").notNull(),
});

/** Map an owned host type onto its rows. */
function hostMapping(policy: Policy, table: typeof articles): TableMapping {
    return {
        policy,
        table,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: { owner: { column: "owner", scope: "global" } },
    };
}

/** Where each type's objects live. */
const mappings: TableMapping[] = [
    hostMapping(article, articles),
    hostMapping(photo, photos),
    {
        policy: remark,
        table: remarks,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            parent: {
                column: "parentId",
                subject: {
                    packageId: "parentPackageId",
                    type: "parentType",
                    scope: "scope",
                },
            },
        },
    },
];

test.for(TEST_DIALECTS)(
    "admit a remark exactly when the caller may read its parent, of whichever type it is, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "attachment",
                tables: [...ACCESS_TABLES, articles, photos, remarks],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const { database } = storage;
        const authorizer = new Authorizer([article, photo, remark], mappings);

        // give alice an article and a photo, bob an article, and remark on each
        await database.insert(articles).values([
            { id: "a1", scope: "personal", owner: "alice" },
            { id: "a2", scope: "personal", owner: "bob" },
        ]);
        await database.insert(photos).values([{ id: "p1", scope: "personal", owner: "alice" }]);
        const packageId = module1.package.id;
        await database.insert(remarks).values([
            {
                id: "r1",
                scope: "personal",
                parentPackageId: packageId,
                parentType: "article",
                parentId: "a1",
            },
            {
                id: "r2",
                scope: "personal",
                parentPackageId: packageId,
                parentType: "photo",
                parentId: "p1",
            },
            {
                id: "r3",
                scope: "personal",
                parentPackageId: packageId,
                parentType: "article",
                parentId: "a2",
            },
            {
                id: "r4",
                scope: "personal",
                parentPackageId: packageId,
                parentType: "photo",
                parentId: "a1",
            },
        ]);

        // decide each remark in memory and by query, which agree
        const alice: AccessContext = {
            subjects: [principal.user.reference("global", "alice")],
            now: 1000,
            attributes: {},
        };
        const authorization = new Authorization(authorizer, database, () => alice);
        const decided = [];
        for (const id of ["r1", "r2", "r3", "r4"]) {
            const decision = await authorization.check(
                remark.permission("read"),
                remark.reference("personal", id),
            );
            decided.push([id, decision.isAllowed]);
        }
        const queried = await database
            .select({ id: remarks.id })
            .from(remarks)
            .where(await authorization.where(remark.permission("read"), "personal", remarks))
            .orderBy(asc(remarks.id));

        // admit the remarks on her own article and photo, never bob's article or a photo that does not exist
        expect(decided).toEqual([
            ["r1", true],
            ["r2", true],
            ["r3", false],
            ["r4", false],
        ]);
        expect(queried.map((row) => row.id)).toEqual(["r1", "r2"]);
    },
);

test("refuse a contribution to a relation that is not open", () => {
    const closed = new Policy(module1.package, {
        name: "closed",
        relations: { parent: { subjects: [principal.user], grantedBy: null } },
        permissions: {},
    });
    const host = new Policy(module1.package, {
        name: "host",
        permissions: {},
        contributes: [{ policy: closed, relation: "parent" }],
    });

    // refuse at assembly, naming the relation
    expect(() => new Authorizer([closed, host])).toThrow(
        "host contributes to closed.parent, which is not an open relation",
    );
});

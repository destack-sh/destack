import { expect, test } from "@destack/test";
import * as turso from "../../turso/index.ts";
import * as postgres from "../../postgres/index.ts";
import { migrate } from "../../migration/index.ts";
import { sql } from "../../index.ts";
import { baseSchema, treeSchema, tree, node } from "./schema.ts";

/** Preserve existing forests while introducing and maintaining an ancestor index. */
test("migrate, query, move and remove scoped trees", async () => {
    const address = process.env.DESTACK_TEST_POSTGRES;
    const administration = address ? await postgres.connect(address) : undefined;
    const name = `tree_${crypto.randomUUID().replaceAll("-", "")}`;
    const url = address ? new URL(address) : undefined;
    if (administration && url) {
        await administration.execute(sql`CREATE DATABASE ${sql.identifier(name)}`);
        url.pathname = `/${name}`;
    }
    const database = url
        ? await postgres.connect(url.href, treeSchema)
        : await turso.connect(":memory:", treeSchema);

    try {
        // populate ordinary parent records before the tree declaration exists
        await migrate(database, baseSchema);
        await database.insert(node).values([
            { id: "a", scope: "one", parent: null },
            { id: "b", scope: "one", parent: "a" },
            { id: "c", scope: "one", parent: "b" },
            { id: "d", scope: "one", parent: null },
            { id: "e", scope: "two", parent: null },
        ]);

        // reject a missing parent without retaining the new index or migration history
        await database.execute(sql`UPDATE ${node} SET parent = 'missing' WHERE id = 'b'`);
        await expect(migrate(database, treeSchema)).rejects.toMatchObject({
            code: "MIGRATION_FAILED",
            cause: { code: "INVALID_MIGRATION", message: "tree parent is missing" },
        });
        expect(await database.select().from(node).orderBy(node.id)).toEqual([
            { id: "a", scope: "one", parent: null },
            { id: "b", scope: "one", parent: "missing" },
            { id: "c", scope: "one", parent: "b" },
            { id: "d", scope: "one", parent: null },
            { id: "e", scope: "two", parent: null },
        ]);

        // reject a cycle and then apply the same history after repairing the source
        await database.execute(sql`UPDATE ${node} SET parent = 'c' WHERE id = 'b'`);
        await expect(migrate(database, treeSchema)).rejects.toMatchObject({
            code: "MIGRATION_FAILED",
            cause: { code: "INVALID_MIGRATION", message: "tree contains a cycle" },
        });
        await database.execute(sql`UPDATE ${node} SET parent = 'a' WHERE id = 'b'`);
        await migrate(database, treeSchema);
        await migrate(database, treeSchema);

        // compare the complete generated ancestry, including self paths
        expect(
            await database
                .select()
                .from(tree.ancestors)
                .orderBy(tree.ancestors.scope, tree.ancestors.ancestor, tree.ancestors.descendant),
        ).toEqual([
            { scope: "one", ancestor: "a", descendant: "a", depth: 0 },
            { scope: "one", ancestor: "a", descendant: "b", depth: 1 },
            { scope: "one", ancestor: "a", descendant: "c", depth: 2 },
            { scope: "one", ancestor: "b", descendant: "b", depth: 0 },
            { scope: "one", ancestor: "b", descendant: "c", depth: 1 },
            { scope: "one", ancestor: "c", descendant: "c", depth: 0 },
            { scope: "one", ancestor: "d", descendant: "d", depth: 0 },
            { scope: "two", ancestor: "e", descendant: "e", depth: 0 },
        ]);
        expect(
            await database
                .select({ id: node.id })
                .from(node)
                .where(tree.roots("one"))
                .orderBy(node.id),
        ).toEqual([{ id: "a" }, { id: "d" }]);
        expect(
            await database.select({ id: node.id }).from(node).where(tree.children("one", "a")),
        ).toEqual([{ id: "b" }]);
        expect(
            await database
                .select({ id: node.id })
                .from(node)
                .where(tree.ancestorsOf("one", "c"))
                .orderBy(node.id),
        ).toEqual([{ id: "a" }, { id: "b" }]);

        // retain inherited paths after a move and reparenting deletion
        await tree.move("one", "b", "d", database);
        expect(
            await database
                .select({ id: node.id })
                .from(node)
                .where(tree.descendantsOf("one", "d"))
                .orderBy(node.id),
        ).toEqual([{ id: "b" }, { id: "c" }]);
        await tree.remove("one", "b", "reparent", database);
        expect(await database.select().from(node).orderBy(node.id)).toEqual([
            { id: "a", scope: "one", parent: null },
            { id: "c", scope: "one", parent: "d" },
            { id: "d", scope: "one", parent: null },
            { id: "e", scope: "two", parent: null },
        ]);

        // delete a complete subtree without touching another scope
        await tree.remove("one", "d", "subtree", database);
        await tree.remove("one", "a", "restrict", database);
        expect(await database.select().from(node)).toEqual([
            { id: "e", scope: "two", parent: null },
        ]);
        expect(await database.select().from(tree.ancestors)).toEqual([
            { scope: "two", ancestor: "e", descendant: "e", depth: 0 },
        ]);
    } finally {
        await database.close();
        if (administration) {
            await administration.execute(sql`DROP DATABASE ${sql.identifier(name)}`);
            await administration.close();
        }
    }
}, 5000);

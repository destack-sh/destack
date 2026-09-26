import { expect, onTestFinished, test } from "@destack/test";
import { migrate } from "../../migration/index.ts";
import { sql } from "../../index.ts";
import { TEST_DIALECTS, TestDatabase } from "../../test/database.ts";
import { baseNode, tree, node } from "./fixture.ts";

/** Preserve existing forests while introducing and maintaining an ancestor index. */
test.for(TEST_DIALECTS)("migrate, query, move and remove scoped trees on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [node]);
    onTestFinished(() => test.close());
    const { database } = test;

    // populate ordinary parent records before the tree declaration exists
    await migrate(database, [baseNode]);
    await database.insert(node).values([
        { id: "a", scope: "one", parent: null },
        { id: "b", scope: "one", parent: "a" },
        { id: "c", scope: "one", parent: "b" },
        { id: "d", scope: "one", parent: null },
        { id: "e", scope: "two", parent: null },
    ]);

    // reject a missing parent without retaining the new index or migration history
    await database.execute(sql`UPDATE ${node} SET parent = 'missing' WHERE id = 'b'`);
    await expect(migrate(database, [node])).rejects.toMatchObject({
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
    await expect(migrate(database, [node])).rejects.toMatchObject({
        code: "MIGRATION_FAILED",
        cause: { code: "INVALID_MIGRATION", message: "tree contains a cycle" },
    });
    await database.execute(sql`UPDATE ${node} SET parent = 'a' WHERE id = 'b'`);
    await migrate(database, [node]);
    await migrate(database, [node]);

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
        await database.select({ id: node.id }).from(node).where(tree.roots("one")).orderBy(node.id),
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
    expect(await database.select().from(node)).toEqual([{ id: "e", scope: "two", parent: null }]);
    expect(await database.select().from(tree.ancestors)).toEqual([
        { scope: "two", ancestor: "e", descendant: "e", depth: 0 },
    ]);
});

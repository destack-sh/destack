import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { TestObjectStore } from "../src/test/index.ts";
import { notebook, notesPackage } from "./fixture/note.ts";
import { space } from "./fixture/space.ts";
import { users } from "./fixture/user.ts";

test("serve a package's objects in a new space, make an example's calls as a user, and refuse an example it cannot make", async () => {
    // serve the notes in a space of the fixture's space type
    await using objects = await TestObjectStore.open({
        packages: [notesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000200"),
    });

    // make alice's notebook through the example's calls, then settle the controllers
    await objects.given(
        {
            objects: {
                space: [{ object: notebook, method: "create", input: { name: "Research" } }],
            },
        },
        users.alice,
    );
    await objects.settle();

    // read it as its owner alone, and refuse a scope or a type the store does not serve
    const names = async (user: typeof users.alice) =>
        (
            await objects.client(notesPackage, user).notebook.list({ spaceId: objects.scope })
        ).items.map((item) => item.name);
    const refused = (example: Parameters<typeof objects.given>[0]) =>
        objects.given(example, users.alice).then(
            () => "made",
            (error: unknown) => (error instanceof Error ? error.message : "unknown"),
        );
    expect([
        await names(users.alice),
        await names(users.bob),
        await refused({ objects: { account: [] } }),
        await refused({
            objects: { space: [{ object: { name: "ledger" }, method: "create", input: {} }] },
        }),
    ]).toEqual([
        ["Research"],
        [],
        "the test store serves its space alone, not the example's account",
        "the test store serves no object type ledger",
    ]);
});

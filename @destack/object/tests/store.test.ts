import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { PackageHandle } from "../src/index.ts";
import { type TestCaller, TestObjectStore } from "../src/test/index.ts";
import { note, notebook, notesPackage } from "./fixture/note.ts";
import { space } from "./fixture/space.ts";
import { users } from "./fixture/user.ts";

test("serve a package's objects in a new space, make an example's calls as a user, and refuse an example it cannot make", async () => {
    // serve the notes in a space of the fixture's space type
    await using objects = await TestObjectStore.open({
        packages: [notesPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000200"),
    });

    // make alice's notebook through the example's calls, then settle the controllers
    await objects.as(users.alice).given({
        objects: {
            space: [{ object: notebook, method: "create", input: { name: "Research" } }],
        },
    });
    await objects.settle();

    // read it as its owner alone, and refuse a scope or a type the store does not serve
    const names = async (user: typeof users.alice) =>
        (
            await objects.as(user).client(notesPackage).notebook.list({ spaceId: objects.scope })
        ).items.map((item) => item.name);
    const given = (example: Parameters<TestCaller["given"]>[0]) =>
        objects.as(users.alice).given(example);
    expect([await names(users.alice), await names(users.bob)]).toEqual([["Research"], []]);
    await expect(given({ objects: { account: [] } })).rejects.toThrow(
        new TypeError("the test store serves its space alone, not the example's account"),
    );
    await expect(
        given({
            objects: { space: [{ object: { name: "ledger" }, method: "create", input: {} }] },
        }),
    ).rejects.toThrow(new TypeError("the test store serves no object type ledger"));
});

test("refuse a package served twice in one store", async () => {
    // serve the notes package's notebooks and notes through two servers
    const opening = TestObjectStore.open({
        packages: [
            PackageHandle.of(notebook.package, { objects: { notebook } }),
            PackageHandle.of(notebook.package, { objects: { note } }),
        ],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000202"),
    });
    await expect(opening).rejects.toThrow(
        new TypeError(`package ${notesPackage.name} is served twice`),
    );
});

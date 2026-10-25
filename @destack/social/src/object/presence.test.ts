import type { Selection } from "@destack/object";
import { article, type TestFollower, TestSocial } from "../test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { presence } from "./index.ts";

test("show an article's readers who is on it once per writer, what they do and where, until they leave", async () => {
    await using kit = await TestSocial.open();
    const { scope, actors } = kit;
    const draft = await kit.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.bob,
        }),
    );

    // let the owner edit the article while the viewer follows who is on it
    const alice = await kit.follow("alice");
    expect(await rows(alice)).toEqual([]);
    const bob = await kit.follow("bob");
    expect(await rows(bob)).toEqual([]);
    const editing = await kit.call(
        "alice",
        presence.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            status: "editing",
            selection: cursor(3),
        }),
    );
    expect([await rows(alice), await rows(bob)]).toEqual([
        [[actors.alice.id, "editing", cursor(3)]],
        [[actors.alice.id, "editing", cursor(3)]],
    ]);

    // show the owner typing as the cursor moves
    await kit.call(
        "alice",
        presence.call("update", {
            ...scope,
            id: editing.id,
            selection: cursor(9),
            status: "typing",
        }),
    );
    expect([await rows(alice), await rows(bob)]).toEqual([
        [[actors.alice.id, "typing", cursor(9)]],
        [[actors.alice.id, "typing", cursor(9)]],
    ]);

    // refuse presence to a stranger, and show the stranger nobody
    expect(
        await refusal(
            kit.call(
                "carol",
                presence.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    status: "viewing",
                }),
            ),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `article ${draft.id} not found`]);
    const carol = await kit.follow("carol");
    expect(await rows(carol)).toEqual([]);

    // let the viewer join, and only its own client move its presence
    const viewing = await kit.call(
        "bob",
        presence.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            status: "viewing",
        }),
    );
    const joined = [
        [actors.alice.id, "typing", cursor(9)],
        [actors.bob.id, "viewing", null],
    ];
    expect([await rows(alice), await rows(bob)]).toEqual([joined, joined]);
    expect(
        await refusal(
            kit.call(
                "alice",
                presence.call("update", { ...scope, id: viewing.id, status: "idle" }),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: write"]);

    // show the viewer alone once the owner leaves
    await kit.call("alice", presence.call("delete", { ...scope, id: editing.id }));
    expect(await rows(bob)).toEqual([[actors.bob.id, "viewing", null]]);

    // refuse a principal's second presence from one writer
    expect(
        await refusal(
            kit.call(
                "bob",
                presence.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    status: "idle",
                }),
            ),
        ),
    ).toEqual(["CONFLICT", "DUPLICATE", "another presence has the same parent, principal, and writerId"]);

    // let another principal reuse the writer only for its own row
    const reused = await kit.call(
        "bob",
        presence.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            status: "idle",
        }),
        "writer-alice",
    );
    expect(await rows(bob)).toEqual([
        [actors.bob.id, "viewing", null],
        [actors.bob.id, "idle", null],
    ]);
    expect(
        await refusal(
            kit.call(
                "alice",
                presence.call("update", { ...scope, id: reused.id, status: "editing" }),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: write"]);
});

/** Read a follower's next rows as principal, status and selection, in a stable order. */
async function rows(client: TestFollower) {
    const read = await client.rows();

    return read.map((row) => [row["principalId"], row["status"], row["selection"]]);
}

/** Select a collapsed point of the body after an offset in alice's first run. */
function cursor(offset: number): Selection {
    const at = { element: { run: "alice.1", offset }, side: "after" } as const;

    return { field: "body", anchor: at, head: at };
}

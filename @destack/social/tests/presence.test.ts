import type { Selection } from "@destack/object";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, refusal, test } from "@destack/test";
import { presence } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, type Follower, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "show an article's readers who is on it once per writer, what they do and where, until they leave, on %s",
    async (dialect) => {
        const { call, host, as, follow, scope } = await serveArticles(dialect);
        const draft = await call(article.call("create", { ...scope, title: "Launch plan" }));
        await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "viewer",
                subject: actors.bob,
            }),
        );

        // let the owner edit the article while the viewer follows who is on it
        const alice = follow();
        expect(await rows(alice)).toEqual([]);
        as("bob");
        const bob = follow();
        expect(await rows(bob)).toEqual([]);
        as("alice");
        const editing = await call(
            presence.call("create", {
                ...scope,
                ...host(article, draft.id),
                status: "editing",
                selection: cursor(3),
            }),
        );
        expect([await rows(alice), await rows(bob)]).toEqual([
            [[actors.alice.id, "editing", cursor(3), false]],
            [[actors.alice.id, "editing", cursor(3), false]],
        ]);

        // show the owner typing as the cursor moves
        await call(
            presence.call("update", {
                ...scope,
                id: editing.id,
                selection: cursor(9),
                isTyping: true,
            }),
        );
        expect([await rows(alice), await rows(bob)]).toEqual([
            [[actors.alice.id, "editing", cursor(9), true]],
            [[actors.alice.id, "editing", cursor(9), true]],
        ]);

        // refuse presence to a stranger, and show the stranger nobody
        as("carol");
        expect(
            await refusal(
                call(
                    presence.call("create", {
                        ...scope,
                        ...host(article, draft.id),
                        status: "viewing",
                    }),
                ),
            ),
        ).toEqual(["NOT_FOUND", `no article ${draft.id}`]);
        const carol = follow();
        expect(await rows(carol)).toEqual([]);

        // let the viewer join, and only its own client move its presence
        as("bob");
        const viewing = await call(
            presence.call("create", { ...scope, ...host(article, draft.id), status: "viewing" }),
        );
        const joined = [
            [actors.alice.id, "editing", cursor(9), true],
            [actors.bob.id, "viewing", null, false],
        ];
        expect([await rows(alice), await rows(bob)]).toEqual([joined, joined]);
        as("alice");
        expect(
            await refusal(
                call(presence.call("update", { ...scope, id: viewing.id, status: "idle" })),
            ),
        ).toEqual(["FORBIDDEN", "permission denied: write"]);

        // show the viewer alone once the owner leaves
        await call(presence.call("delete", { ...scope, id: editing.id }));
        expect(await rows(bob)).toEqual([[actors.bob.id, "viewing", null, false]]);

        // refuse a second presence of one principal's writer, and let another principal reuse the writer only for its own row
        as("bob");
        expect(
            await refusal(
                call(
                    presence.call("create", {
                        ...scope,
                        ...host(article, draft.id),
                        status: "idle",
                    }),
                ),
            ),
        ).toEqual(["CONFLICT", "another presence has the same parent, principal, and writerId"]);
        const reused = await call(
            presence.call("create", { ...scope, ...host(article, draft.id), status: "idle" }),
            "writer-alice",
        );
        expect(await rows(bob)).toEqual([
            [actors.bob.id, "viewing", null, false],
            [actors.bob.id, "idle", null, false],
        ]);
        as("alice");
        expect(
            await refusal(
                call(presence.call("update", { ...scope, id: reused.id, status: "editing" })),
            ),
        ).toEqual(["FORBIDDEN", "permission denied: write"]);
    },
);

/** Read a follower's next rows as principal, status, selection and typing. */
async function rows(client: Follower) {
    return (await client.rows()).map((row) => [
        row["principalId"],
        row["status"],
        row["selection"],
        row["isTyping"],
    ]);
}

/** Select a collapsed point of the body after an offset in alice's first run. */
function cursor(offset: number): Selection {
    const at = { element: { run: "alice.1", offset }, side: "after" } as const;

    return { field: "body", anchor: at, head: at };
}

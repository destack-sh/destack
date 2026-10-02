import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { presence } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles, refused } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "show an article's readers who is on it once per client, what they do and where, until they leave, on %s",
    async (dialect) => {
        const { call, host, as, follow } = await serveArticles(dialect);
        const draft = await call(article, "create", { title: "Launch plan" });
        await call(article, "grant", { id: draft.id, relation: "viewer", subject: actors.bob });
        const rows = async (client: ReturnType<typeof follow>) =>
            (await client.rows()).map((row) => [
                row.principal,
                row.status,
                row.selection,
                row.isTyping,
            ]);

        // let the owner edit the article while the viewer follows who is on it
        const alice = follow();
        expect(await rows(alice)).toEqual([]);
        as("bob");
        const bob = follow();
        expect(await rows(bob)).toEqual([]);
        as("alice");
        const cursor = (offset: number) => {
            const at = { element: { run: "alice.1", offset }, side: "after" };

            return { field: "body", anchor: at, head: at };
        };
        const editing = await call(presence, "create", {
            ...host(article, draft.id),
            status: "editing",
            selection: cursor(3),
        });
        expect([await rows(alice), await rows(bob)]).toEqual([
            [[Subject.key(actors.alice), "editing", cursor(3), false]],
            [[Subject.key(actors.alice), "editing", cursor(3), false]],
        ]);

        // show the owner typing as the cursor moves
        await call(presence, "update", {
            id: editing.id,
            selection: cursor(9),
            isTyping: true,
        });
        expect([await rows(alice), await rows(bob)]).toEqual([
            [[Subject.key(actors.alice), "editing", cursor(9), true]],
            [[Subject.key(actors.alice), "editing", cursor(9), true]],
        ]);

        // refuse presence to a stranger, and show the stranger nobody
        as("carol");
        expect(
            await refused(
                call(presence, "create", { ...host(article, draft.id), status: "viewing" }),
            ),
        ).toEqual(["NOT_FOUND", `no article ${draft.id}`]);
        const carol = follow();
        expect(await rows(carol)).toEqual([]);

        // let the viewer join, and only its own client move its presence
        as("bob");
        const viewing = await call(presence, "create", {
            ...host(article, draft.id),
            status: "viewing",
        });
        const joined = [
            [Subject.key(actors.alice), "editing", cursor(9), true],
            [Subject.key(actors.bob), "viewing", null, false],
        ];
        expect([await rows(alice), await rows(bob)]).toEqual([joined, joined]);
        as("alice");
        expect(await refused(call(presence, "update", { id: viewing.id, status: "idle" }))).toEqual(
            ["FORBIDDEN", "permission denied: write"],
        );

        // show the viewer alone once the owner leaves
        await call(presence, "delete", { id: editing.id });
        expect(await rows(bob)).toEqual([[Subject.key(actors.bob), "viewing", null, false]]);

        // refuse a second presence of one principal's client, and let another principal copy the client only for its own row
        as("bob");
        expect(
            await refused(call(presence, "create", { ...host(article, draft.id), status: "idle" })),
        ).toEqual(["DUPLICATE", "a record with the same unique key exists"]);
        const copied = await call(presence, "create", {
            ...host(article, draft.id),
            status: "idle",
            client: "client-alice",
        });
        expect(await rows(bob)).toEqual([
            [Subject.key(actors.bob), "viewing", null, false],
            [Subject.key(actors.bob), "idle", null, false],
        ]);
        as("alice");
        expect(
            await refused(call(presence, "update", { id: copied.id, status: "editing" })),
        ).toEqual(["FORBIDDEN", "permission denied: write"]);
    },
);

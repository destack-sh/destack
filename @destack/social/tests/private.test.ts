import { TEST_DIALECTS } from "@destack/db/test";
import { expect, refusal, test } from "@destack/test";
import { favourite, receipt } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "give each reader one receipt and one favourite of an article, private to them, on %s",
    async (dialect) => {
        const { call, list, host, as, scope } = await serveArticles(dialect);
        const draft = await call(article.call("create", { ...scope, title: "Launch plan" }));
        const shared = await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "viewer",
                subject: actors.bob,
            }),
        );

        // let the owner and the viewer each mark one receipt and one favourite
        const mine = await call(receipt.call("create", { ...scope, ...host(article, draft.id) }));
        await call(favourite.call("create", { ...scope, ...host(article, draft.id) }));
        as("bob");
        const theirs = await call(receipt.call("create", { ...scope, ...host(article, draft.id) }));
        await call(favourite.call("create", { ...scope, ...host(article, draft.id) }));
        expect(
            await refusal(call(receipt.call("create", { ...scope, ...host(article, draft.id) }))),
        ).toEqual(["CONFLICT", "another receipt has the same parent and owner"]);
        expect(
            await refusal(call(favourite.call("create", { ...scope, ...host(article, draft.id) }))),
        ).toEqual(["CONFLICT", "another favourite has the same parent and owner"]);

        // move the viewer's receipt and its time on the next read
        const read = await call(receipt.call("update", { ...scope, id: theirs.id }));
        expect([read.revision, read.updatedAt >= theirs.updatedAt]).toEqual([2, true]);

        // show each reader only its own, and refuse the owner's receipt to the viewer
        const owned = async () => [
            (await list(receipt)).map((row) => row.ownerId),
            (await list(favourite)).map((row) => row.ownerId),
        ];
        expect(await owned()).toEqual([[actors.bob.id], [actors.bob.id]]);
        expect(await refusal(call(receipt.call("update", { ...scope, id: mine.id })))).toEqual([
            "NOT_FOUND",
            `no receipt ${mine.id}`,
        ]);
        as("alice");
        expect(await owned()).toEqual([[actors.alice.id], [actors.alice.id]]);

        // hide the viewer's own after it loses the article
        await call(article.call("revoke", { ...scope, id: draft.id, relationshipId: shared.id }));
        as("bob");
        expect(await owned()).toEqual([[], []]);
    },
);

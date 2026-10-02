import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { favourite, receipt } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles, refused } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "keep each reader's receipt and favourite of an article single and private to them, on %s",
    async (dialect) => {
        const { call, list, host, as } = await serveArticles(dialect);
        const draft = await call(article, "create", { title: "Launch plan" });
        const shared = await call(article, "grant", {
            id: draft.id,
            relation: "viewer",
            subject: actors.bob,
        });

        // let the owner and the viewer each keep one receipt and one favourite
        const mine = await call(receipt, "create", host(article, draft.id));
        await call(favourite, "create", host(article, draft.id));
        as("bob");
        const theirs = await call(receipt, "create", host(article, draft.id));
        await call(favourite, "create", host(article, draft.id));
        expect(await refused(call(receipt, "create", host(article, draft.id)))).toEqual([
            "DUPLICATE",
            "a record with the same unique key exists",
        ]);
        expect(await refused(call(favourite, "create", host(article, draft.id)))).toEqual([
            "DUPLICATE",
            "a record with the same unique key exists",
        ]);

        // move the viewer's receipt and its time on the next read
        const read = await call(receipt, "update", { id: theirs.id });
        expect([read.revision, (read.updatedAt as number) >= (theirs.updatedAt as number)]).toEqual(
            [2, true],
        );

        // show each reader only its own, and refuse the owner's receipt to the viewer
        const owned = async () => [
            (await list(receipt)).map((row) => row.owner),
            (await list(favourite)).map((row) => row.owner),
        ];
        expect(await owned()).toEqual([[Subject.key(actors.bob)], [Subject.key(actors.bob)]]);
        expect(await refused(call(receipt, "update", { id: mine.id }))).toEqual([
            "NOT_FOUND",
            `no receipt ${mine.id}`,
        ]);
        as("alice");
        expect(await owned()).toEqual([[Subject.key(actors.alice)], [Subject.key(actors.alice)]]);

        // hide the viewer's own after it loses the article
        await call(article, "revoke", { id: draft.id, relationshipId: shared.id });
        as("bob");
        expect(await owned()).toEqual([[], []]);
    },
);

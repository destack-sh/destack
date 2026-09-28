import { subjectKey } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { favourite, receipt } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles } from "./fixture/server.ts";

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

        // let the owner and the viewer each hold one receipt and one favourite
        const mine = await call(receipt, "create", host(article, draft.id));
        await call(favourite, "create", host(article, draft.id));
        as("bob");
        const theirs = await call(receipt, "create", host(article, draft.id));
        await call(favourite, "create", host(article, draft.id));
        await expect(call(receipt, "create", host(article, draft.id))).rejects.toMatchObject({
            code: "DUPLICATE",
            message: "a record with the same unique key exists",
        });
        await expect(call(favourite, "create", host(article, draft.id))).rejects.toMatchObject({
            code: "DUPLICATE",
            message: "a record with the same unique key exists",
        });

        // move the viewer's receipt as it reads again, stamping when it read
        const read = await call(receipt, "update", { id: theirs.id });
        expect([read.revision, (read.updatedAt as number) >= (theirs.updatedAt as number)]).toEqual(
            [2, true],
        );

        // show each reader only its own, and refuse the owner's receipt to the viewer
        const owned = async () => [
            (await list(receipt)).map((row) => row.owner),
            (await list(favourite)).map((row) => row.owner),
        ];
        expect(await owned()).toEqual([[subjectKey(actors.bob)], [subjectKey(actors.bob)]]);
        await expect(call(receipt, "update", { id: mine.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "Not Found",
        });
        as("alice");
        expect(await owned()).toEqual([[subjectKey(actors.alice)], [subjectKey(actors.alice)]]);

        // hide the viewer's own once it loses the article
        await call(article, "revoke", { id: draft.id, relationshipId: shared.id });
        as("bob");
        expect(await owned()).toEqual([[], []]);
    },
);

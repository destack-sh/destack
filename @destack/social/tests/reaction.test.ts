import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { comment, reaction } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "let commenters react once per emoji to articles and their comments, counted on each, on %s",
    async (dialect) => {
        const { call, list, host, as } = await serveArticles(dialect);

        // share an article with a commenter, an agent commenting on it and a viewer, and start a thread
        const draft = await call(article, "create", { title: "Launch plan" });
        await call(article, "grant", { id: draft.id, relation: "commenter", subject: actors.bob });
        await call(article, "grant", {
            id: draft.id,
            relation: "commenter",
            subject: actors.agent,
        });
        await call(article, "grant", { id: draft.id, relation: "viewer", subject: actors.carol });
        const first = await call(comment, "create", {
            ...host(article, draft.id),
            body: { text: "Ship it?", mentions: [] },
        });

        // let the commenter react to the article once per emoji, and the agent too
        as("bob");
        const thumbs = await call(reaction, "create", { ...host(article, draft.id), emoji: "👍" });
        expect(
            await refusal(call(reaction, "create", { ...host(article, draft.id), emoji: "👍" })),
        ).toEqual(["DUPLICATE", "a record with the same unique key exists"]);
        await call(reaction, "create", { ...host(article, draft.id), emoji: "👨🏻‍❤️‍💋‍👨🏼" });
        expect(
            await refusal(call(reaction, "create", { ...host(article, draft.id), emoji: "yes" })),
        ).toEqual(["BAD_REQUEST", "invalid input to reaction.create"]);
        as("agent");
        await call(reaction, "create", { ...host(article, draft.id), emoji: "👍" });
        await call(reaction, "create", { ...host(comment, first.id), emoji: "🎉" });

        // refuse the viewer a reaction to the article and to its comment, since reactions follow commenting
        as("carol");
        expect(
            await refusal(call(reaction, "create", { ...host(article, draft.id), emoji: "🎉" })),
        ).toEqual(["FORBIDDEN", "permission denied: react"]);
        expect(
            await refusal(call(reaction, "create", { ...host(comment, first.id), emoji: "🎉" })),
        ).toEqual(["FORBIDDEN", "permission denied: react"]);

        // count the reactions on the article and on the comment
        expect([
            (await call(article, "get", { id: draft.id })).reactionCount,
            (await call(comment, "get", { id: first.id })).reactionCount,
        ]).toEqual([3, 1]);

        // take back a reaction only as its author
        as("agent");
        expect(await refusal(call(reaction, "delete", { id: thumbs.id }))).toEqual([
            "FORBIDDEN",
            "permission denied: write",
        ]);
        as("bob");
        await call(reaction, "delete", { id: thumbs.id });
        expect(
            (await list(reaction)).map((row) => [row.author, row.parentType, row.emoji]),
        ).toEqual([
            [Subject.key(actors.bob), "article", "👨🏻‍❤️‍💋‍👨🏼"],
            [Subject.key(actors.agent), "article", "👍"],
            [Subject.key(actors.agent), "comment", "🎉"],
        ]);
        expect((await call(article, "get", { id: draft.id })).reactionCount).toBe(2);
    },
);

import { subjectKey } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { comment } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "thread comments on an article, which commenters reply to, authors and editors resolve, and owners moderate, on %s",
    async (dialect) => {
        const { call, list, host, as } = await serveArticles(dialect);

        // share an article with an editor, a commenter and a viewer
        const draft = await call(article, "create", { title: "Launch plan" });
        await call(article, "grant", { id: draft.id, relation: "editor", subject: actors.dave });
        await call(article, "grant", { id: draft.id, relation: "commenter", subject: actors.bob });
        await call(article, "grant", { id: draft.id, relation: "viewer", subject: actors.carol });

        // start a thread on a selection of the article's text, and let the commenter reply to it
        await call(article, "edit", {
            id: draft.id,
            field: "body",
            edits: [{ insert: "Our launch starts Monday.", run: "alice.1" }],
        });
        const selection = {
            field: "body",
            anchor: { element: { run: "alice.1", offset: 0 }, side: "before" },
            head: { element: { run: "alice.1", offset: 9 }, side: "after" },
        };
        const first = await call(comment, "create", {
            ...host(article, draft.id),
            body: { text: "Tighten the intro", mentions: [] },
            selection,
        });
        as("bob");
        const reply = await call(comment, "create", {
            ...host(article, draft.id),
            body: { text: "Done", mentions: [] },
            thread: first.id,
        });

        // show the viewer the thread, and refuse the viewer a comment
        as("carol");
        expect(
            (await list(comment)).map((row) => [row.author, row.thread, row.body, row.selection]),
        ).toEqual([
            [
                subjectKey(actors.alice),
                null,
                { text: "Tighten the intro", mentions: [] },
                selection,
            ],
            [subjectKey(actors.bob), first.id, { text: "Done", mentions: [] }, null],
        ]);
        await expect(
            call(comment, "create", {
                ...host(article, draft.id),
                body: { text: "Nope", mentions: [] },
            }),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: comment",
        });

        // refuse replies to a reply, to a thread on another host, and replies with a selection of their own
        as("alice");
        const other = await call(article, "create", { title: "Retro" });
        await call(article, "grant", { id: other.id, relation: "commenter", subject: actors.bob });
        as("bob");
        await expect(
            call(comment, "create", {
                ...host(article, other.id),
                body: { text: "Nope", mentions: [] },
                thread: first.id,
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: "thread not found on the host" });
        await expect(
            call(comment, "create", {
                ...host(article, draft.id),
                body: { text: "Nope", mentions: [] },
                thread: reply.id,
            }),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "a reply answers the first comment of its thread",
        });
        await expect(
            call(comment, "create", {
                ...host(article, draft.id),
                body: { text: "Nope", mentions: [] },
                selection,
                thread: first.id,
            }),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "a reply takes its thread's selection",
        });

        // refuse a selection of elements the text never held
        await expect(
            call(comment, "create", {
                ...host(article, draft.id),
                body: { text: "Nope", mentions: [] },
                selection: {
                    ...selection,
                    head: { ...selection.head, element: { run: "bob.9", offset: 0 } },
                },
            }),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "body holds no such selection",
        });

        // let only the author edit a comment, stamping the edit
        const edited = await call(comment, "update", {
            id: reply.id,
            body: { text: "Done, see the new intro", mentions: [] },
        });
        expect([edited.body, typeof edited.editedAt, first.editedAt]).toEqual([
            { text: "Done, see the new intro", mentions: [] },
            "number",
            null,
        ]);
        as("alice");
        await expect(
            call(comment, "update", { id: reply.id, body: { text: "Mine now", mentions: [] } }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "Forbidden" });

        // refuse resolving to a commenter who wrote no part of the thread's first comment
        as("bob");
        await expect(call(comment, "resolve", { id: first.id })).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "Forbidden",
        });

        // let an editor resolve the thread through its first comment once, and its author reopen it
        as("dave");
        const resolved = await call(comment, "resolve", { id: first.id });
        expect([typeof resolved.resolvedAt, resolved.resolvedBy]).toEqual([
            "number",
            subjectKey(actors.dave),
        ]);
        await expect(call(comment, "resolve", { id: first.id })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "thread is already resolved",
        });
        await expect(call(comment, "resolve", { id: reply.id })).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "a thread resolves through its first comment",
        });
        as("alice");
        const reopened = await call(comment, "reopen", { id: first.id });
        expect([reopened.resolvedAt, reopened.resolvedBy]).toEqual([null, null]);

        // let the owner delete another's comment, and refuse a commenter another's
        expect((await call(article, "get", { id: draft.id })).commentCount).toBe(2);
        await call(comment, "delete", { id: reply.id });
        await call(comment, "create", {
            ...host(article, draft.id),
            body: { text: "Reworded", mentions: [] },
            thread: first.id,
        });
        as("bob");
        await expect(call(comment, "delete", { id: first.id })).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "Forbidden",
        });

        // count the article's comments, and delete the thread's replies with its first comment
        as("alice");
        expect((await call(article, "get", { id: draft.id })).commentCount).toBe(2);
        await call(comment, "delete", { id: first.id });
        expect([
            await list(comment),
            (await call(article, "get", { id: draft.id })).commentCount,
        ]).toEqual([[], 0]);
    },
);

test.for(TEST_DIALECTS)(
    "refuse mentions outside the text, across each other or of non-principals, on %s",
    async (dialect) => {
        const { call, host, spaceId } = await serveArticles(dialect);
        const draft = await call(article, "create", { title: "Launch plan" });
        const text = "@bob and @agent, please review";
        const mention = (mentions: readonly object[]) =>
            call(comment, "create", { ...host(article, draft.id), body: { text, mentions } });

        // accept ordered, disjoint spans within the text
        const created = await mention([
            { offset: 0, length: 4, principal: actors.bob },
            { offset: 9, length: 6, principal: actors.agent },
        ]);
        expect(created.body).toEqual({
            text,
            mentions: [
                { offset: 0, length: 4, principal: actors.bob },
                { offset: 9, length: 6, principal: actors.agent },
            ],
        });

        // refuse a span past the text, overlapping spans and spans out of order
        const refusals = await Promise.all(
            [
                [{ offset: 28, length: 4, principal: actors.bob }],
                [
                    { offset: 0, length: 4, principal: actors.bob },
                    { offset: 3, length: 6, principal: actors.agent },
                ],
                [
                    { offset: 9, length: 6, principal: actors.agent },
                    { offset: 0, length: 4, principal: actors.bob },
                ],
            ].map((mentions) => mention(mentions).catch((error: unknown) => error)),
        );
        expect(refusals).toMatchObject([
            {
                code: "BAD_REQUEST",
                message: "mentions name ordered, disjoint spans within the text",
            },
            {
                code: "BAD_REQUEST",
                message: "mentions name ordered, disjoint spans within the text",
            },
            {
                code: "BAD_REQUEST",
                message: "mentions name ordered, disjoint spans within the text",
            },
        ]);

        // refuse a mention of an object other than a principal
        await expect(
            mention([{ offset: 0, length: 4, principal: article.reference(spaceId, draft.id) }]),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "invalid input to comment.create",
        });
    },
);

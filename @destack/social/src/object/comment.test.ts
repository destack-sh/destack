import type { Selection } from "@destack/object";
import { article, TestSocial } from "../test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { Comment, comment } from "./index.ts";

test("thread comments on an article, which commenters reply to, authors and editors resolve, and owners moderate", async () => {
    await using fixture = await TestSocial.open();
    const { scope, actors } = fixture;

    // share an article with an editor, a commenter and a viewer
    const draft = await fixture.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "editor",
            subject: actors.dave,
        }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.carol,
        }),
    );

    // start a thread on a selection of the article's text, and let the commenter reply to it
    await fixture.call(
        "alice",
        article.call("edit", {
            ...scope,
            id: draft.id,
            field: "body",
            edits: [{ insert: "Our launch starts Monday.", run: "alice.1" }],
        }),
    );
    const selection: Selection = {
        field: "body",
        anchor: { element: { run: "alice.1", offset: 0 }, side: "before" },
        head: { element: { run: "alice.1", offset: 9 }, side: "after" },
    };
    const first = await fixture.call(
        "alice",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: { text: "Tighten the intro", mentions: [] },
            selection,
        }),
    );
    const reply = await fixture.call(
        "bob",
        comment.call("reply", { ...scope, id: first.id, text: "Done" }),
    );

    // show the viewer the thread, and refuse the viewer a comment
    expect(
        (await fixture.list("carol", comment)).map((row) => [
            row.body,
            row.authorId,
            row.threadId,
            row.selection,
        ]),
    ).toEqual([
        [{ text: "Tighten the intro", mentions: [] }, actors.alice.id, null, selection],
        [{ text: "Done", mentions: [] }, actors.bob.id, first.id, null],
    ]);
    expect(
        await refusal(
            fixture.call(
                "carol",
                comment.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    body: { text: "Nope", mentions: [] },
                }),
            ),
        ),
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'comment' is forbidden: the caller lacks it",
    ]);

    // refuse a reply to a reply, and a reply's mentions past the end of its text
    const nested = fixture.call(
        "bob",
        comment.call("reply", { ...scope, id: reply.id, text: "Nope" }),
    );
    const unspanned = fixture.call(
        "bob",
        comment.call("reply", {
            ...scope,
            id: first.id,
            text: "Nope",
            mentions: [{ offset: 2, length: 6, principal: actors.alice }],
        }),
    );
    expect(await Promise.all([refusal(nested), refusal(unspanned)])).toEqual([
        [
            "BAD_REQUEST",
            "FIRST_COMMENT_REQUIRED",
            "thread's first comment is required: only it takes replies and resolves the thread",
        ],
        [
            "BAD_REQUEST",
            "MENTIONS_INVALID",
            "mention list is invalid: mentions must be ordered, disjoint spans within the text",
        ],
    ]);

    // refuse a selection of elements the text never had
    expect(
        await refusal(
            fixture.call(
                "bob",
                comment.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    body: { text: "Nope", mentions: [] },
                    selection: {
                        ...selection,
                        head: {
                            side: selection.head.side,
                            element: { run: "bob.9", offset: 0 },
                        },
                    },
                }),
            ),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "SELECTION_INVALID",
        "selection in 'body' is invalid: the host text has no such selection",
    ]);

    // refuse a selection of a field with no text, and mentions past the end of the text
    const titled = fixture.call(
        "bob",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: { text: "Nope", mentions: [] },
            selection: { ...selection, field: "title" },
        }),
    );
    const overlong = fixture.call(
        "bob",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: { text: "Nope", mentions: [{ offset: 2, length: 6, principal: actors.alice }] },
        }),
    );
    expect(await Promise.all([refusal(titled), refusal(overlong)])).toEqual([
        [
            "BAD_REQUEST",
            "TEXT_FIELD_INVALID",
            "text field 'title' is invalid: article has no such text field",
        ],
        [
            "BAD_REQUEST",
            "MENTIONS_INVALID",
            "mention list is invalid: mentions must be ordered, disjoint spans within the text",
        ],
    ]);

    // let only the author edit a comment, stamping the edit
    const editedAt = fixture.clock.now();
    const edited = await fixture.call(
        "bob",
        comment.call("update", {
            ...scope,
            id: reply.id,
            body: { text: "Done, see the new intro", mentions: [] },
        }),
    );
    expect([edited.body, edited.editedAt, first.editedAt]).toEqual([
        { text: "Done, see the new intro", mentions: [] },
        editedAt,
        null,
    ]);
    expect(
        await refusal(
            fixture.call(
                "alice",
                comment.call("update", {
                    ...scope,
                    id: reply.id,
                    body: { text: "Mine now", mentions: [] },
                }),
            ),
        ),
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'write' is forbidden: the caller lacks it",
    ]);

    // refuse a commenter resolving another's thread
    expect(
        await refusal(fixture.call("bob", comment.call("resolve", { ...scope, id: first.id }))),
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'resolve' is forbidden: the caller lacks it",
    ]);

    // let an editor resolve the thread through its first comment once, and its author reopen it
    const resolvedAt = fixture.clock.now();
    const resolved = await fixture.call(
        "dave",
        comment.call("resolve", { ...scope, id: first.id }),
    );
    expect(new Comment(resolved).resolution).toEqual({ at: resolvedAt, by: actors.dave });
    expect(
        await refusal(fixture.call("dave", comment.call("resolve", { ...scope, id: first.id }))),
    ).toEqual([
        "PRECONDITION_FAILED",
        "THREAD_BLOCKED",
        "cannot resolve the thread: it is already resolved",
    ]);
    expect(
        await refusal(fixture.call("dave", comment.call("resolve", { ...scope, id: reply.id }))),
    ).toEqual([
        "BAD_REQUEST",
        "FIRST_COMMENT_REQUIRED",
        "thread's first comment is required: only it takes replies and resolves the thread",
    ]);
    const reopened = await fixture.call(
        "alice",
        comment.call("reopen", { ...scope, id: first.id }),
    );
    expect(new Comment(reopened).resolution).toBeUndefined();
    expect(
        await refusal(fixture.call("alice", comment.call("reopen", { ...scope, id: first.id }))),
    ).toEqual([
        "PRECONDITION_FAILED",
        "THREAD_BLOCKED",
        "cannot reopen the thread: it is already open",
    ]);

    // let the owner delete another's comment, and refuse a commenter another's
    expect(
        (await fixture.call("alice", article.call("get", { ...scope, id: draft.id }))).commentCount,
    ).toBe(2);
    await fixture.call("alice", comment.call("delete", { ...scope, id: reply.id }));
    await fixture.call(
        "alice",
        comment.call("reply", { ...scope, id: first.id, text: "Reworded" }),
    );
    expect(
        await refusal(fixture.call("bob", comment.call("delete", { ...scope, id: first.id }))),
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'delete' is forbidden: the caller lacks it",
    ]);

    // count the article's comments, and delete the thread's replies with its first comment
    expect(
        (await fixture.call("alice", article.call("get", { ...scope, id: draft.id }))).commentCount,
    ).toBe(2);
    await fixture.call("alice", comment.call("delete", { ...scope, id: first.id }));
    expect([
        await fixture.list("alice", comment),
        (await fixture.call("alice", article.call("get", { ...scope, id: draft.id }))).commentCount,
    ]).toEqual([[], 0]);
});

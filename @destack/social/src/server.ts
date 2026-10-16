import { defineServer } from "@destack/object/server";
import { CallError } from "@destack/runtime/error";
import { ObjectReference, Subject } from "@destack/sync";
import { Comment, comment, reply, thread } from "./object/comment.ts";
import socialPackage from "./package.ts";

/** The social package's server, whose comments notify the principals they mention. */
export default defineServer(socialPackage, {
    objects: [
        comment.implement({
            methods: {
                create: async (context, next) => {
                    // validate mentions, then the thread, then the selection
                    const { body, threadId, selection } = context.input;
                    Comment.requireSpans(body);
                    if (threadId !== null && threadId !== undefined) {
                        await Comment.requireThread(context, threadId, selection);
                    } else if (selection !== null && selection !== undefined) {
                        await Comment.requireSelection(context, selection);
                    }

                    // subscribe the author, notify the mentioned and announce the comment to the others
                    const row = await next();
                    const isReply = row.threadId !== null;
                    const mentioned = Comment.mentioned(row);
                    const author = context.caller.subject;
                    await context.mutate(context.caller, [
                        Comment.subscribe(
                            Comment.thread(row),
                            author,
                            isReply ? "participating" : "author",
                        ),
                        ...Comment.mention(row, mentioned),
                        (isReply ? reply : thread).call({
                            source: Comment.source(row),
                            audience: { kind: "subscribers" },
                            thread: row.parentId,
                            payload: Comment.excerpt(row),
                            excluded: mentioned,
                        }),
                    ]);

                    return row;
                },

                resolve: async (context) => {
                    // refuse a resolved thread
                    const target = Comment.requireFirst(context.target);
                    if (target.resolvedAt !== null) {
                        throw new CallError("CONFLICT", { message: "thread is already resolved" });
                    }

                    return context.update({
                        resolvedAt: context.now,
                        ...ObjectReference.columns("resolvedBy", context.caller.subject),
                    });
                },

                reopen: async (context) => {
                    // refuse an open thread
                    const target = Comment.requireFirst(context.target);
                    if (target.resolvedAt === null) {
                        throw new CallError("CONFLICT", { message: "thread is open" });
                    }

                    return context.update({
                        resolvedAt: null,
                        ...ObjectReference.columns("resolvedBy", undefined),
                    });
                },

                reply: async (context) => {
                    // create the reply on the thread's host, answering with it
                    const target = Comment.requireFirst(context.target);
                    const host = ObjectReference.parent(target);
                    const [created] = await context.mutate(context.caller, [
                        comment.call("create", {
                            parent: { packageId: host.packageId, type: host.type, id: host.id },
                            body: { text: context.input.text, mentions: [] },
                            threadId: target.id,
                        }),
                    ]);

                    return created;
                },

                update: async (context, next) => {
                    // validate a changed body and stamp the edit
                    const { body } = context.input;
                    if (body !== undefined) {
                        Comment.requireSpans(body);
                    }
                    const edited = context.with({
                        input: { ...context.input, editedAt: context.now },
                    });

                    // notify the principals mentioned for the first time
                    const before = Comment.mentioned(context.target);
                    const row = await next(body === undefined ? context : edited);
                    const mentioned = Comment.mentioned(row).filter(
                        (principal) => !before.some((known) => Subject.same(known, principal)),
                    );
                    await context.mutate(context.caller, Comment.mention(row, mentioned));

                    return row;
                },
            },
        }),
    ],
});

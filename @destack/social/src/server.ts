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
                create: async (call, next) => {
                    // validate mentions, then the thread, then the selection
                    const { body, threadId, selection } = call.input;
                    Comment.requireSpans(body);
                    if (threadId !== null && threadId !== undefined) {
                        await Comment.requireThread(call, threadId, selection);
                    } else if (selection !== null && selection !== undefined) {
                        await Comment.requireSelection(call, selection);
                    }

                    // subscribe the author, notify the mentioned and announce the comment to the others
                    const row = await next();
                    const isReply = row.threadId !== null;
                    const mentioned = Comment.mentioned(row);
                    const author = call.requireCaller();
                    await call.mutate(call.caller, [
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

                resolve: async (call) => {
                    // refuse a resolved thread
                    const target = Comment.requireFirst(call.target);
                    if (target.resolvedAt !== null) {
                        throw new CallError("CONFLICT", { message: "thread is already resolved" });
                    }

                    return call.update({
                        resolvedAt: call.now,
                        resolvedBy: Subject.key(call.requireCaller()),
                    });
                },

                reopen: async (call) => {
                    // refuse an open thread
                    const target = Comment.requireFirst(call.target);
                    if (target.resolvedAt === null) {
                        throw new CallError("CONFLICT", { message: "thread is open" });
                    }

                    return call.update({ resolvedAt: null, resolvedBy: null });
                },

                reply: async (call) => {
                    // create the reply on the thread's host, answering with it
                    const target = Comment.requireFirst(call.target);
                    const host = ObjectReference.parent(target);
                    const [created] = await call.mutate(call.caller, [
                        comment.call("create", {
                            parent: { packageId: host.packageId, type: host.type, id: host.id },
                            body: { text: call.input.text, mentions: [] },
                            threadId: target.id,
                        }),
                    ]);

                    return created;
                },

                update: async (call, next) => {
                    // validate a changed body and stamp the edit
                    const { body } = call.input;
                    if (body !== undefined) {
                        Comment.requireSpans(body);
                    }
                    const edited = call.with({ input: { ...call.input, editedAt: call.now } });

                    // notify the principals mentioned for the first time
                    const before = Comment.mentioned(call.target);
                    const row = await next(body === undefined ? call : edited);
                    const mentioned = Comment.mentioned(row).filter(
                        (principal) => !before.some((known) => Subject.same(known, principal)),
                    );
                    await call.mutate(call.caller, Comment.mention(row, mentioned));

                    return row;
                },
            },
        }),
    ],
});

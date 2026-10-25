import type { MethodContext } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { ObjectReference, Subject } from "@destack/sync";
import { Comment, comment } from "./object/comment.ts";
import { favourite, LOCATION_BYTES, presence, receipt } from "./object/index.ts";
import socialPackage from "./package.ts";
import { SocialError } from "./error/index.ts";

/** The social package's server, whose comments subscribe and notify their participants. */
export default implementPackage(socialPackage, {
    objects: [
        comment.implement({
            methods: {
                create: async (context, next) => {
                    // validate the mentions and any selection
                    const { selection, body } = context.input;
                    Comment.requireSpans(body);
                    if (selection !== undefined && selection !== null) {
                        await Comment.requireSelection(context, selection);
                    }

                    // announce the comment
                    const row = await next();
                    await context.mutate(context.caller, new Comment(row).announce());

                    return row;
                },

                resolve: async (context) => {
                    // refuse a resolved thread
                    const first = new Comment(context.target).requireFirst();
                    if (first.resolution !== undefined) {
                        throw SocialError.of("THREAD_RESOLVED", "thread is already resolved");
                    }

                    return context.update({
                        resolvedAt: context.now,
                        ...ObjectReference.columns("resolvedBy", context.caller.subject),
                    });
                },

                reopen: async (context) => {
                    // refuse an open thread
                    const first = new Comment(context.target).requireFirst();
                    if (first.resolution === undefined) {
                        throw SocialError.of("THREAD_OPEN", "thread is already open");
                    }

                    return context.update({
                        resolvedAt: null,
                        ...ObjectReference.columns("resolvedBy", undefined),
                    });
                },

                reply: async (context) => {
                    // validate the reply's mentions on the thread's first comment
                    const { row: target } = new Comment(context.target).requireFirst();
                    const { text, mentions } = context.input;
                    const body = { text, mentions };
                    Comment.requireSpans(body);

                    // insert the reply on the thread's host and announce it
                    const row = await context.insert(comment, {
                        parentPackageId: target.parentPackageId,
                        parentType: target.parentType,
                        parentId: target.parentId,
                        body,
                        threadId: target.id,
                    });
                    await context.mutate(context.caller, new Comment(row).announce());

                    return row;
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
                    const before = new Comment(context.target).mentioned();
                    const updated = new Comment(await next(body === undefined ? context : edited));
                    const mentioned = updated
                        .mentioned()
                        .filter(
                            (principal) => !before.some((known) => Subject.same(known, principal)),
                        );
                    const calls = updated.mention(mentioned);
                    if (calls.length > 0) {
                        await context.mutate(context.caller, calls);
                    }

                    return updated.row;
                },
            },
        }),
        receipt.implement({
            methods: {
                create: async (context, next) => {
                    // move an existing mark to now, or mark the object read
                    const [existing] = await context
                        .read(receipt)
                        .findMany({ where: owned(context) });
                    if (existing === undefined) {
                        return next();
                    }
                    const [moved] = await context.mutate(context.caller, [
                        receipt.call("update", receipt.target(existing)),
                    ]);

                    return moved;
                },
            },
        }),
        favourite.implement({
            methods: {
                create: async (context, next) => {
                    // answer an existing star, or star the object
                    const [existing] = await context
                        .read(favourite)
                        .findMany({ where: owned(context) });

                    return existing ?? next();
                },
            },
        }),
        presence.implement({
            methods: {
                create: async (context, next) => {
                    // refuse a large place outside text
                    requireLocation(context.input.location);

                    return next();
                },
                update: async (context, next) => {
                    // refuse a large place outside text
                    requireLocation(context.input.location);

                    return next();
                },
            },
        }),
    ],
});

/** Select the caller's own row of a per-principal attachment on the call's parent. */
function owned(context: MethodContext) {
    const parent = context.requireParent();

    return {
        parentPackageId: parent.packageId,
        parentType: parent.type,
        parentId: parent.id,
        ownerId: context.caller.subject.id,
    };
}

/** Refuse a place outside text past its bytes. */
function requireLocation(location: unknown): void {
    const bytes = new TextEncoder().encode(JSON.stringify(location ?? null)).length;
    if (bytes > LOCATION_BYTES) {
        throw SocialError.of(
            "LOCATION_TOO_LARGE",
            `presence locations take at most ${LOCATION_BYTES} bytes`,
        );
    }
}

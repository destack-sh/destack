import { type MethodContext, textChunk, TextSpan } from "@destack/object";
import { schema } from "@destack/schema";
import { implementPackage } from "@destack/object/server";
import { implementWorkflow } from "@destack/workflow/server";
import { principal } from "@destack/access";
import { ObjectReference } from "@destack/sync";
import { Comment, comment } from "./object/comment.ts";
import { mentioned, mentioning, PASSAGE_LENGTH } from "./object/mention.ts";
import { favourite, LOCATION_BYTES, presence, receipt } from "./object/index.ts";
import socialPackage from "./package.ts";
import { SocialError } from "./error/index.ts";

/** The spans a comment's creation gives its body. */
const BODY = schema.array(TextSpan);

/** A page of a text's chunks, as the mention notices quote them. */
const PIECES = schema.looseObject({
    items: schema.array(schema.looseObject({ text: schema.string() })),
});

/** The principal kinds a mention notifies, as only principals read notices. */
const PRINCIPALS = [principal.user, principal.machine, principal.installation];

/** Notify each principal a text newly mentions once, with the start of the text. */
const mentions = implementWorkflow(mentioning, {
    run: async (step, input, run) => {
        // notify only a principal, as an object reads no notices, from the editor the run acts for
        const { subject, parent, field, scope } = input;
        const author = run.actor;
        if (!PRINCIPALS.some((kind) => kind.is(subject))) {
            return;
        }

        // quote the start of the text that mentions it
        const pieces = await step.query(
            "text",
            textChunk.call("list", {
                ...textChunk.in(scope),
                where: { parentId: parent.id, field },
                orderBy: { position: "asc" },
            }),
        );
        const text = PIECES.parse(pieces)
            .items.map((piece) => piece.text)
            .join("")
            .slice(0, PASSAGE_LENGTH);

        // notify it from the text's owner, a comment's from its thread, where its readers alone hear of it
        const owner = { ...parent, scope };
        const isComment = parent.packageId === comment.package.id && parent.type === comment.name;
        const notice = isComment
            ? Comment.notice(
                  await step.query("comment", comment.call("get", comment.target(owner))),
              )
            : { source: owner, thread: parent.id };
        await step.call(
            "notify",
            mentioned.call({
                ...notice,
                recipients: [subject],
                reason: "mention",
                payload: { author, text },
            }),
        );
    },
});

/** The social package's server, whose comments subscribe and notify their participants. */
export default implementPackage(socialPackage, {
    objects: [
        ...mentions.objects,
        comment.implement({
            methods: {
                create: async (context, next) => {
                    // validate any selection
                    const { selection } = context.input;
                    if (selection !== undefined && selection !== null) {
                        await Comment.requireSelection(context, selection);
                    }

                    // announce the comment with the start of its body
                    const row = await next();
                    const body = BODY.parse(context.input["body"] ?? []);
                    await context.mutate(context.caller, new Comment(row).announce(body));

                    return row;
                },

                resolve: async (context) => {
                    // refuse a resolved thread
                    const first = new Comment(context.target).requireFirst();
                    if (first.resolution !== undefined) {
                        throw SocialError.of(
                            "THREAD_BLOCKED",
                            "cannot resolve the thread: it is already resolved",
                        );
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
                        throw SocialError.of(
                            "THREAD_BLOCKED",
                            "cannot reopen the thread: it is already open",
                        );
                    }

                    return context.update({
                        resolvedAt: null,
                        ...ObjectReference.columns("resolvedBy", undefined),
                    });
                },

                reply: async (context) => {
                    // insert the reply on the thread's host and announce it
                    const { row: target } = new Comment(context.target).requireFirst();
                    const { body } = context.input;
                    const row = await context.insert(
                        comment,
                        {
                            parentPackageId: target.parentPackageId,
                            parentType: target.parentType,
                            parentId: target.parentId,
                            threadId: target.id,
                        },
                        { body },
                    );
                    await context.mutate(context.caller, new Comment(row).announce(body));

                    return row;
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
            "LOCATION_INVALID",
            `presence location is invalid: it must take at most ${LOCATION_BYTES} bytes`,
        );
    }
}

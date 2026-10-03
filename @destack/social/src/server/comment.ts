import { type ObjectReference, Subject } from "@destack/sync";
import { and, eq } from "@destack/db";
import { Subscription } from "@destack/notification";
import { type Call, Chunk, type Selection } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import * as base from "../object/comment.ts";

/** Comments on the server that subscribe and notify participants. */
export const comment = base.comment.handle({
    create: async (call, next) => {
        // validate mentions, the selection and the thread
        const input = call.input;
        requireSpans(input.body);
        if (input.selection !== null && input.selection !== undefined) {
            await requireText(call, input.selection);
        }
        if (input.threadId !== null && input.threadId !== undefined) {
            await requireThreadOf(call, input.threadId);
        }

        // subscribe the author and notify the mentioned
        const row = await next();
        const isReply = row.threadId !== null;
        const mentioned = row.body.mentions.map((mention) => mention.principal);
        await Subscription.add(
            call,
            rootOf(row),
            call.requireCaller(),
            isReply ? "participating" : "author",
        );
        await notifyMentioned(call, row, mentioned);

        // announce the comment to the other subscribers
        const notice = {
            source: placeOf(row),
            audience: { kind: "subscribers" },
            thread: row.parentId,
            payload: excerptOf(row),
            excluded: mentioned,
        } as const;
        await (isReply ? base.reply : base.thread).announce(call, notice);

        return row;
    },
    update: async (call, next) => {
        // validate a changed body and stamp the edit
        const { body } = call.input;
        if (body !== undefined) {
            requireSpans(body);
        }
        const edited = call.with({ input: { ...call.input, editedAt: call.now } });

        // notify principals mentioned for the first time
        const before = call.target.body.mentions.map((mention) => mention.principal);
        const row = await next(body === undefined ? call : edited);
        const mentioned = row.body.mentions
            .map((mention) => mention.principal)
            .filter((principal) => !before.some((known) => Subject.same(known, principal)));
        await notifyMentioned(call, row, mentioned);

        return row;
    },
    resolve: async (call) => {
        // resolve an open thread as the calling principal
        const target = requireFirst(call.target);
        if (target.resolvedAt !== null) {
            throw new ServiceError("CONFLICT", { message: "thread is already resolved" });
        }

        return call.update({ resolvedAt: call.now, resolvedBy: Subject.key(call.requireCaller()) });
    },
    reopen: async (call) => {
        // reopen a resolved thread
        const target = requireFirst(call.target);
        if (target.resolvedAt === null) {
            throw new ServiceError("CONFLICT", { message: "thread is open" });
        }

        return call.update({ resolvedAt: null, resolvedBy: null });
    },
});

/** Require ordered, disjoint mention spans within the text. */
function requireSpans(body: base.Body): void {
    let end = 0;
    for (const mention of body.mentions) {
        if (mention.offset < end || mention.offset + mention.length > body.text.length) {
            throw new ServiceError("BAD_REQUEST", {
                message: "mentions must be ordered, disjoint spans within the text",
            });
        }
        end = mention.offset + mention.length;
    }
}

/** Require a selection within a text field of the comment's host. */
async function requireText(call: Call, selection: Selection): Promise<void> {
    // find the host's type and the selected elements
    const host = call.requireParent();
    const type = call.objects.find((object) => object.policy.is(host));
    const elements = [selection.anchor.element, selection.head.element];

    // refuse unknown fields and elements
    if (type === undefined || !type.text.includes(selection.field)) {
        throw new ServiceError("BAD_REQUEST", {
            message: `${host.type} has no text field ${selection.field}`,
        });
    } else if (!(await Chunk.contains(call.database, host, selection.field, elements))) {
        throw new ServiceError("BAD_REQUEST", {
            message: `${selection.field} has no such selection`,
        });
    }
}

/** Require a reply to answer a thread's first comment on the same host. */
async function requireThreadOf(
    call: Call<typeof base.comment.table>,
    thread: base.Comment["id"],
): Promise<void> {
    // read the thread's first comment on the same host
    const table = base.comment.table;
    const host = call.requireParent();
    const [first] = await call.database
        .select({ thread: table.threadId })
        .from(table)
        .where(
            and(
                eq(table.id, thread),
                eq(table.parentPackageId, host.packageId),
                eq(table.parentType, host.type),
                eq(table.parentId, host.id),
            ),
        );

    // refuse a missing thread, a nested reply and a reply's own selection
    if (first === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "thread not found on the host" });
    } else if (first.thread !== null) {
        throw new ServiceError("BAD_REQUEST", {
            message: "a reply answers the first comment of its thread",
        });
    } else if (call.input["selection"] !== undefined) {
        throw new ServiceError("BAD_REQUEST", { message: "a reply takes its thread's selection" });
    }
}

/** Require a comment to be the first of its thread, returning it. */
function requireFirst(target: base.Comment): base.Comment {
    if (target.threadId !== null) {
        throw new ServiceError("BAD_REQUEST", {
            message: "a thread resolves through its first comment",
        });
    }

    return target;
}

/** Reference a comment's host. */
function hostOf(row: base.Comment): ObjectReference {
    return {
        packageId: row.parentPackageId,
        type: row.parentType,
        scope: row.scope,
        id: row.parentId,
    };
}

/** Reference a comment's thread. */
function rootOf(row: base.Comment): ObjectReference {
    return base.comment.reference(row.scope, row.threadId ?? row.id);
}

/** Reference where a comment's notifications go: its thread for a reply, else its host. */
function placeOf(row: base.Comment): ObjectReference {
    return row.threadId === null ? hostOf(row) : rootOf(row);
}

/** Build a comment's excerpt without splitting a surrogate pair. */
function excerptOf(row: base.Comment): base.Excerpt {
    const text = row.body.text.slice(0, base.EXCERPT_LENGTH);
    const isSplit = /[\uD800-\uDBFF]$/u.test(text);

    return {
        author: Subject.read(row.author),
        comment: row.id,
        thread: row.threadId ?? row.id,
        text: isSplit ? text.slice(0, -1) : text,
    };
}

/** Subscribe and notify the principals a comment mentions. */
async function notifyMentioned(
    call: Call,
    row: base.Comment,
    mentioned: readonly Subject[],
): Promise<void> {
    // subscribe each principal
    const place = placeOf(row);
    for (const principal of mentioned) {
        await Subscription.add(call, place, principal, "mention");
    }

    // notify them at once
    await base.mention.notify(call, {
        source: place,
        recipients: mentioned,
        reason: "mention",
        thread: row.parentId,
        payload: excerptOf(row),
    });
}

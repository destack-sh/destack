import { ObjectReference, Subject } from "@destack/sync";
import { type Reason, subscription } from "@destack/notification";
import { type Call, Chunk, type Selection } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import * as base from "../object/comment.ts";

/** Comments on the server that subscribe and notify participants. */
export const comment = base.comment.implement({
    methods: {
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

            // subscribe the author, notify the mentioned and announce the comment to the others
            const row = await next();
            const isReply = row.threadId !== null;
            const mentioned = row.body.mentions.map((mention) => mention.principal);
            const author = call.requireCaller();
            await call.mutate(call.caller, [
                subscribe(rootOf(row), author, isReply ? "participating" : "author"),
                ...mentionCalls(row, mentioned),
                (isReply ? base.reply : base.thread).call({
                    source: placeOf(row),
                    audience: { kind: "subscribers" },
                    thread: row.parentId,
                    payload: excerptOf(row),
                    excluded: mentioned,
                }),
            ]);

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
            await call.mutate(call.caller, mentionCalls(row, mentioned));

            return row;
        },
        resolve: async (call) => {
            // resolve an open thread as the calling principal
            const target = requireFirst(call.target);
            if (target.resolvedAt !== null) {
                throw new ServiceError("CONFLICT", { message: "thread is already resolved" });
            }

            return call.update({
                resolvedAt: call.now,
                resolvedBy: Subject.key(call.requireCaller()),
            });
        },
        reopen: async (call) => {
            // reopen a resolved thread
            const target = requireFirst(call.target);
            if (target.resolvedAt === null) {
                throw new ServiceError("CONFLICT", { message: "thread is open" });
            }

            return call.update({ resolvedAt: null, resolvedBy: null });
        },
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
    } else if (
        !(await Chunk.contains(call.requireTransaction(), host, selection.field, elements))
    ) {
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
    const host = call.requireParent();
    const [first] = await call.read(base.comment).rows({
        id: thread,
        parentPackageId: host.packageId,
        parentType: host.type,
        parentId: host.id,
    });

    // refuse a missing thread, a nested reply and a reply's own selection
    if (first === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "thread not found on the host" });
    } else if (first.threadId !== null) {
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

/** Reference a comment's thread. */
function rootOf(row: base.Comment): ObjectReference {
    return base.comment.reference(row.scope, row.threadId ?? row.id);
}

/** Reference where a comment's notifications go: its thread for a reply, else its host. */
function placeOf(row: base.Comment): ObjectReference {
    return row.threadId === null ? ObjectReference.parent(row) : rootOf(row);
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

/** Build the calls subscribing and notifying the principals a comment mentions. */
function mentionCalls(row: base.Comment, mentioned: readonly Subject[]) {
    const place = placeOf(row);

    return [
        ...mentioned.map((principal) => subscribe(place, principal, "mention")),
        base.mention.call({
            source: place,
            recipients: mentioned,
            reason: "mention",
            thread: row.parentId,
            payload: excerptOf(row),
        }),
    ];
}

/** Build the call subscribing a principal to an object for a reason. */
function subscribe(host: ObjectReference, principal: Subject, reason: Reason) {
    return subscription.call("add", {
        parent: { packageId: host.packageId, type: host.type, id: host.id },
        owner: Subject.key(principal),
        reason,
    });
}

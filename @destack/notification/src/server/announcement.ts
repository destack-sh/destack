import { principal } from "@destack/access";
import type { MethodContext, MethodContextOf, ResultOf } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema, Text } from "@destack/schema";
import { ObjectReference, Scope, Subject } from "@destack/sync";
import { announcement } from "../object/announcement.ts";
import { type Reason, subscription } from "../object/subscription.ts";
import type { NotificationServer } from "./server.ts";

/** One addressee of an announcement's batch, with the reason it is notified. */
interface Addressee {
    /** The position of the person in its audience, which the cursor records. */
    readonly position: string;
    /** The person. */
    readonly owner: Subject;
    /** Why the person is notified. */
    readonly reason: Reason;
}

/** A page of an audience: after a position, up to a count. */
interface Page {
    /** The position the page starts after, absent for the first. */
    readonly after?: string;
    /** The most recipients of the page. */
    readonly limit: number;
}

/** Implement announcements: post or replace them, and expand them into activities. */
export function implementAnnouncement(server: NotificationServer): typeof announcement {
    return announcement.implement({
        methods: {
            announce: async (call) =>
                (await server.notification(call, call.input.packageId, call.input.name)).announce(
                    call,
                ),
            expand: (context) => expand(context, server),
        },
        controller: {
            // expand a thread's announcements together, in the order they were posted
            pending: { expandedAt: { isNull: true } },
            key: (row) => ({ thread: row.thread }),
            reconcile: async (reconciliation) => {
                const posted = reconciliation.rows.toSorted(
                    (left, right) =>
                        left.postedAt - right.postedAt || Text.compare(left.id, right.id),
                );
                await reconciliation.mutate(
                    posted.map((row) => announcement.call("expand", announcement.target(row))),
                );

                return undefined;
            },
        },
    });
}

/** Expand an announcement's next batch into its recipients' activities. */
async function expand(
    context: MethodContextOf<typeof announcement, "expand">,
    server: NotificationServer,
): Promise<ResultOf<typeof announcement, "expand">> {
    // read the source and the page after the cursor
    const row = context.target;
    const source = ObjectReference.parent(row);
    const limit = server.batch;
    const page = { ...(row.cursor === null ? {} : { after: row.cursor }), limit };

    // read the next batch of the audience, skipping the author, the serving code without one, and excluded principals
    const audience = await readAudience(context, source, page);
    const actor =
        row.authorId === null ? context.caller.subject : ObjectReference.at("author", row);
    const skipped = new Set([Subject.key(actor), ...row.excluded]);
    const recipients = audience.filter((entry) => !skipped.has(Subject.key(entry.owner)));

    // notify each reason's recipients as of the announcement's post
    const reasons = [...Map.groupBy(recipients, (entry) => entry.reason)];
    for (const [reason, entries] of reasons) {
        const notice = {
            parent: { packageId: source.packageId, type: source.type, id: source.id },
            packageId: row.packageId,
            name: row.name,
            recipients: entries.map((entry) => entry.owner),
            reason,
            payload: row.payload,
            ...(row.key === null ? {} : { key: row.key }),
            thread: row.thread,
        };
        await server.notify(context, notice, { occurredAt: row.postedAt, actor });
    }

    // advance the cursor, finishing with a batch short of full
    return context.update({
        cursor: audience.at(-1)?.position ?? row.cursor,
        expandedAt: audience.length < limit ? context.now : null,
    });
}

/** Read a page of an announcement's audience. */
async function readAudience(
    context: MethodContextOf<typeof announcement, "expand">,
    source: ObjectReference,
    page: Page,
): Promise<readonly Addressee[]> {
    // read the announcement's audience
    const row = context.target;
    const target = row.audience;

    // read the source's subscribers, oldest first, as subscribed when it was announced
    if (target.kind === "subscribers") {
        const rows = await context.read(subscription).findMany({
            where: {
                parentPackageId: source.packageId,
                parentType: source.type,
                parentId: source.id,
                ...(page.after === undefined
                    ? {}
                    : { id: { gt: schema.identifier("subscription").parse(page.after) } }),
                createdAt: { lte: row.postedAt },
            },
            orderBy: { id: "asc" },
            limit: page.limit,
        });

        return rows.map((entry) => ({
            position: entry.id,
            owner: ObjectReference.at("owner", entry),
            reason: entry.reason,
        }));
    }
    // read the space's members, the users reading it, for the announcement's reason
    else if (target.kind === "members") {
        const served = await Scope.object(context, context.scope);
        const members = await readPermitted(context, served, "read", page);

        return members.map((entry) => ({ ...entry, reason: target.reason }));
    }
    // read the permitted users for the announcement's reason
    else {
        const permitted = await readPermitted(context, source, target.permission, page);

        return permitted.map((entry) => ({ ...entry, reason: target.reason }));
    }
}

/** Read a page of the users with a permission on an object. */
async function readPermitted(
    context: MethodContext,
    source: ObjectReference,
    permission: string,
    page: Page,
): Promise<readonly { readonly position: string; readonly owner: Subject }[]> {
    // list the user principals with the permission
    const subjects = await context.lookup(
        { packageId: PackageId.parse(source.packageId), type: source.type, name: permission },
        source,
        {
            type: { packageId: principal.user.definition.packageId, type: principal.user.name },
            page,
        },
    );

    return subjects.map((owner) => ({ position: Subject.key(owner), owner }));
}

import { principal } from "@destack/access";
import type { MethodContext, MethodContextOf, ResultOf } from "@destack/object";
import { PackageId } from "@destack/package";
import { ObjectReference, Scope, Subject } from "@destack/sync";
import { activity } from "../object/activity.ts";
import { announcement } from "../object/announcement.ts";
import { type Reason, subscription } from "../object/subscription.ts";
import type { NotificationServer } from "./server.ts";

/** One addressee of an announcement's batch, with the reason it is notified. */
interface Addressee {
    /** The position of the person in its audience, which the cursor records. */
    readonly id: string;
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
                (await server.catalog(call, call.input.packageId))
                    .get(call.input.name)
                    .announce(call),
            expand: (context) => expand(context, server.batch),
        },
        controller: {
            pending: { expandedAt: { isNull: true } },
            reconcile: async (reconciliation) => {
                // expand the announcement's next batch
                await reconciliation.mutate(
                    reconciliation.rows.map((row) =>
                        announcement.call("expand", announcement.target(row)),
                    ),
                );

                return undefined;
            },
        },
    });
}

/** Expand an announcement's next batch into its recipients' activities. */
async function expand(
    context: MethodContextOf<typeof announcement, "expand">,
    limit: number,
): Promise<ResultOf<typeof announcement, "expand">> {
    // read the source and the page after the cursor
    const row = context.target;
    const source = ObjectReference.parent(row);
    const page = { ...(row.cursor === null ? {} : { after: row.cursor }), limit };

    // read the next batch of the audience, skipping the author and excluded principals
    const audience = await readAudience(context, source, page);
    const author = row.authorId === null ? [] : [Subject.key(ObjectReference.at("author", row))];
    const skipped = new Set([...author, ...row.excluded]);
    const recipients = audience.filter((entry) => !skipped.has(Subject.key(entry.owner)));

    // notify each reason's recipients together in one mutation
    const reasons = [...Map.groupBy(recipients, (entry) => entry.reason)];
    await context.mutate(
        context.caller,
        reasons.map(([reason, entries]) =>
            activity.call("notify", {
                parent: { packageId: source.packageId, type: source.type, id: source.id },
                packageId: row.packageId,
                name: row.name,
                recipients: entries.map((entry) => entry.owner),
                reason,
                payload: row.payload,
                ...(row.key === null ? {} : { key: row.key }),
                thread: row.thread,
            }),
        ),
    );

    // advance the cursor, finishing with a batch short of full
    return context.update({
        cursor: audience.at(-1)?.id ?? row.cursor,
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

    // read the source's subscribers
    if (target.kind === "subscribers") {
        const parent = { packageId: source.packageId, type: source.type, id: source.id };
        const read = await context.query(
            context.caller,
            subscription.call("subscribers", { parent, ...page, until: row.updatedAt }),
        );

        return read.items;
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
): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
    // list the user principals with the permission
    const subjects = await context.lookup(
        { packageId: PackageId.parse(source.packageId), type: source.type, name: permission },
        source,
        {
            type: { packageId: principal.user.definition.packageId, type: principal.user.name },
            page,
        },
    );

    return subjects.map((owner) => ({ id: Subject.key(owner), owner }));
}

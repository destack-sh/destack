import { principal } from "@destack/access";
import { asc, and, gt, Snapshot } from "@destack/db";
import { type Call, type CallOf, type ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { Replica, Scope, type ObjectReference, Subject } from "@destack/sync";
import { user } from "@destack/account/object";
import type { NotificationType } from "../notification/notification.ts";
import { activity, type Activity } from "../object/activity.ts";
import { announcement } from "../object/announcement.ts";
import { type Reason, Subscription } from "../object/subscription.ts";

/** The recipients of one announcement batch by default, as many as one notify call names. */
const BATCH = 100;

/** One recipient of an announcement's batch, with the reason it is notified. */
interface Recipient {
    /** The position of the recipient in its audience, which the cursor names. */
    readonly id: string;
    /** The recipient. */
    readonly owner: Subject;
    /** Why the recipient is notified. */
    readonly reason: Reason;
}

/** What an app serves its activities with: its declared notifications. */
export interface NotificationOptions {
    /** The declarations whose actions answer the activities and whose announcements expand. */
    readonly notifications: readonly NotificationType[];
    /** The recipients of one announcement batch, 100 by default. */
    readonly batch?: number;
}

/** Serve an app's activities and announcements, answering the activities' actions and expanding the announcements under their controller. */
export function serveNotifications(options: NotificationOptions) {
    return {
        activity: activity.handle({ act: (call) => act(call, options) }),
        announcement: announcement.handle({ expand: (call) => expand(call, options) }).control({
            pending: { expandedAt: { isNull: true } },
            reconcile: async (reconciliation) => {
                // expand the announcement's next batch
                await reconciliation.execute("expand", reconciliation.rows);

                return undefined;
            },
        }),
    };
}

/** Find the served declaration of an activity or announcement row. */
function declarationOf(
    row: Pick<Activity, "packageId" | "name">,
    options: NotificationOptions,
): NotificationType {
    const declaration = options.notifications.find((declared) => declared.is(row));
    if (declaration === undefined) {
        throw new TypeError(`notification ${row.packageId}/${row.name} is not served`);
    }

    return declaration;
}

/** Answer an activity's action as its recipient. */
function act(
    call: CallOf<typeof activity, "act">,
    options: NotificationOptions,
): Promise<ResultOf<typeof activity, "act">> {
    const { action, text } = call.input;

    return declarationOf(call.target, options).act(call, action, text);
}

/** Expand an announcement's next batch into its recipients' activities. */
async function expand(
    call: CallOf<typeof announcement, "expand">,
    options: NotificationOptions,
): Promise<ResultOf<typeof announcement, "expand">> {
    // name the source and the page after the cursor
    const row = call.requireTarget();
    const source: ObjectReference = {
        packageId: row.parentPackageId,
        type: row.parentType,
        scope: row.scope,
        id: row.parentId,
    };
    const limit = options.batch ?? BATCH;
    const page = { ...(row.cursor === null ? {} : { after: row.cursor }), limit };

    // read the next batch of the audience, skipping the author and excluded principals
    const audience = await readAudience(call, source, page);
    const skipped = new Set([...(row.author === null ? [] : [row.author]), ...row.excluded]);
    const recipients = audience.filter((entry) => !skipped.has(Subject.key(entry.owner)));

    // notify each reason's recipients together
    const reasons = Map.groupBy(recipients, (entry) => entry.reason);
    for (const [reason, entries] of reasons) {
        await declarationOf(row, options).notify(call, {
            source,
            recipients: entries.map((entry) => entry.owner),
            reason,
            payload: row.payload,
            ...(row.key === null ? {} : { key: row.key }),
            thread: row.thread,
        });
    }

    // advance the cursor, finishing with a batch short of full
    return call.update({
        cursor: audience.at(-1)?.id ?? row.cursor,
        expandedAt: audience.length < limit ? call.now : null,
    });
}

/** Read a page of an announcement's audience: its source's subscribers, the space's members, or the users with a permission. */
async function readAudience(
    call: CallOf<typeof announcement, "expand">,
    source: ObjectReference,
    page: { readonly after?: string; readonly limit: number },
): Promise<readonly Recipient[]> {
    // read the announcement's audience
    const row = call.requireTarget();
    const target = row.audience;

    // read the source's subscribers
    if (target.kind === "subscribers") {
        return Subscription.list(call, source, { ...page, until: row.updatedAt });
    }
    // read the space's members for the announcement's reason
    else if (target.kind === "members") {
        const members = await readMembers(call, page);

        return members.map((entry) => ({ ...entry, reason: target.reason }));
    }

    // read the permitted users for the announcement's reason
    const permitted = await readPermitted(call, source, target.permission, page);

    return permitted.map((entry) => ({ ...entry, reason: target.reason }));
}

/** Read a page of the space's members: the users its copy follows because they joined it. */
async function readMembers(
    call: Call,
    page: { readonly after?: string; readonly limit: number },
): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
    const rows = await call.database
        .select({ id: user.table.id })
        .from(user.table)
        .where(
            and(
                Replica.includes(call.scope, user.table),
                page.after === undefined
                    ? undefined
                    : gt(user.table.id, schema.identifier("user").parse(page.after)),
            ),
        )
        .orderBy(asc(user.table.id))
        .limit(page.limit);

    return rows.map((row) => ({
        id: row.id,
        owner: principal.user.reference(Scope.universe.id, row.id),
    }));
}

/** Read a page of the users with a permission on an object. */
async function readPermitted(
    call: Call,
    source: ObjectReference,
    permission: string,
    page: { readonly after?: string; readonly limit: number },
): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
    // list the user principals with the permission
    const authorizer = call.requireAuthorization().authorizer;
    const userType = {
        packageId: principal.user.definition.packageId,
        type: principal.user.name,
    };
    const subjects = await authorizer.subjects(
        Snapshot.live(call.database),
        authorizer.policy(source).permission(permission),
        source,
        userType,
        call.now,
        page,
    );

    return subjects.map((owner) => ({ id: Subject.key(owner), owner }));
}

import { principal } from "@destack/access";
import { Key, TABLE } from "@destack/db";
import { type Call, type CallOf, type NextOf, type ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference, replicaRow, Scope, Subject } from "@destack/sync";
import { user } from "@destack/account/object";
import type { NotificationType } from "../notification/notification.ts";
import { activity, type Activity } from "../object/activity.ts";
import { announcement } from "../object/announcement.ts";
import { type Reason, subscription } from "../object/subscription.ts";

/** The recipients of one announcement batch by default, as many as one notify call lists. */
const BATCH = 100;

/** One recipient of an announcement's batch, with the reason it is notified. */
interface Recipient {
    /** The position of the recipient in its audience, which the cursor keeps. */
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

/** Serve an app's activities, announcements and subscriptions. */
export function serveActivities(options: NotificationOptions) {
    return {
        activity: activity.implement({
            methods: {
                act: (call) => act(call, options),
                notify: (call) => declarationOf(call.input, options).notify(call),
            },
        }),
        announcement: announcement.implement({
            methods: {
                announce: (call) => declarationOf(call.input, options).announce(call),
                expand: (call) => expand(call, options),
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
        }),
        subscription: subscription.implement({
            methods: {
                add: (call, next) => add(call, next),
                subscribers: (call) => subscribers(call),
            },
        }),
    };
}

/** Find the served declaration of an activity, an announcement or a notice. */
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
    // read the source and the page after the cursor
    const row = call.requireTarget();
    const source = ObjectReference.parent(row);
    const limit = options.batch ?? BATCH;
    const page = { ...(row.cursor === null ? {} : { after: row.cursor }), limit };

    // read the next batch of the audience, skipping the author and excluded principals
    const audience = await readAudience(call, source, page);
    const skipped = new Set([...(row.author === null ? [] : [row.author]), ...row.excluded]);
    const recipients = audience.filter((entry) => !skipped.has(Subject.key(entry.owner)));

    // notify each reason's recipients together in one mutation
    const reasons = [...Map.groupBy(recipients, (entry) => entry.reason)];
    await call.mutate(
        call.caller,
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
    return call.update({
        cursor: audience.at(-1)?.id ?? row.cursor,
        expandedAt: audience.length < limit ? call.now : null,
    });
}

/** Read a page of an announcement's audience. */
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
        const parent = { packageId: source.packageId, type: source.type, id: source.id };
        const read = await call.query(
            call.caller,
            subscription.call("subscribers", { parent, ...page, until: row.updatedAt }),
        );

        return read.items;
    }
    // read the space's members for the announcement's reason
    else if (target.kind === "members") {
        const members = await readMembers(call, page);

        return members.map((entry) => ({ ...entry, reason: target.reason }));
    }
    // read the permitted users for the announcement's reason
    else {
        const permitted = await readPermitted(call, source, target.permission, page);

        return permitted.map((entry) => ({ ...entry, reason: target.reason }));
    }
}

/** Subscribe a principal to an object, keeping an existing subscription. */
async function add(
    call: CallOf<typeof subscription, "add">,
    next: NextOf<typeof subscription, "add">,
): Promise<ResultOf<typeof subscription, "add">> {
    // keep the principal's subscription to the parent, or subscribe it
    const parent = call.requireParent();
    const [existing] = await call.read(subscription).rows({
        parentPackageId: parent.packageId,
        parentType: parent.type,
        parentId: parent.id,
        owner: schema.string().parse(call.input["owner"]),
    });

    return existing ?? next();
}

/** Read a page of an object's subscribers, oldest first. */
async function subscribers(
    call: CallOf<typeof subscription, "subscribers">,
): Promise<ResultOf<typeof subscription, "subscribers">> {
    const { parent, after, limit, until } = call.input;
    const rows = await call.read(subscription).rows(
        {
            parentPackageId: parent.packageId,
            parentType: parent.type,
            parentId: parent.id,
            ...(after === undefined ? {} : { id: { gt: after } }),
            ...(until === undefined ? {} : { createdAt: { lte: until } }),
        },
        { orderBy: { id: "asc" }, limit },
    );

    return {
        items: rows.map((row) => ({
            id: row.id,
            owner: Subject.read(row.owner),
            reason: row.reason,
        })),
    };
}

/** Read a page of the space's members: the users its copy follows because they joined it. */
async function readMembers(
    call: Call,
    page: { readonly after?: string; readonly limit: number },
): Promise<readonly { readonly id: string; readonly owner: Subject }[]> {
    // read the next users the space's copy includes, in key order past the page's start
    const after =
        page.after === undefined ? {} : { key: { gt: Key.name(user.table, { id: page.after }) } };
    const included = await call
        .requireTransaction()
        .rows(
            replicaRow,
            { name: call.scope, table: user.table[TABLE].sqlName, ...after },
            { orderBy: { key: "asc" }, limit: page.limit },
        );

    return included.map((row) => {
        const id = schema.identifier("user").parse(Key.parse(user.table, row.key)["id"]);

        return { id, owner: principal.user.reference(Scope.universe.id, id) };
    });
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
        call.view(),
        authorizer.policy(source).permission(permission),
        source,
        userType,
        call.now,
        page,
    );

    return subjects.map((owner) => ({ id: Subject.key(owner), owner }));
}

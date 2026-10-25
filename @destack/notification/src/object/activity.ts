import { intersection, relation, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, defineShape, field, ParentReference } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { ObjectReference, type ReferenceColumns, Subject } from "@destack/sync";
import { space } from "@destack/account/object";

import { ActionMetadata, ActionName, NotificationContent } from "../notification/content.ts";
import { INTERRUPTION_LEVELS, Preference } from "../preference/preference.ts";
import { REASONS } from "./subscription.ts";

/** The most recipients one notify call lists: 100 inserts of about 0.3 ms. */
export const NOTIFY_RECIPIENTS = 100;

/** The longest key or thread. */
const KEY_LENGTH = 200;

/** A notification's name in its declaring package. */
export const NotificationName = schema.string().regex(/^[a-z][a-zA-Z0-9]*$/u);

/** A notification key or thread. */
export const NotificationKey = schema.string().min(1).max(KEY_LENGTH);

/** What happened to an object, for one person. */
export const activity = defineObject(
    {
        name: "activity",
        plural: "activities",
        scope: space,
        nested: { in: "any", receive: "notify" },
        fields: {
            /** The person it is for. */
            recipient: field.subject().personal(),
            /** The principal who caused it: the notifying caller, or an announcement's author. */
            actor: field.subject().personal(),

            // declaration and identity
            /** The package declaring the notification. */
            packageId: field.string(PackageId),
            /** The notification's name in its package. */
            name: field.string(NotificationName),
            /** The identity a later notice with the same key replaces. */
            key: field.string(NotificationKey).optional(),
            /** The thread it groups in. */
            thread: field.string(NotificationKey),

            // occurrence
            /** Why the recipient hears of it. */
            reason: field.enum(REASONS),
            /** The values its content renders. */
            payload: field.json(schema.json()),
            /** The occurrences collapsed into it. */
            count: field.integer().default(1),
            /** When the first occurrence collapsed into it occurred. */
            firstOccurredAt: field.time(),
            /** When it last occurred. */
            occurredAt: field.time(),

            // rendering in the recipient's locale, as their home shows and delivers it
            /** The text every channel shows, rendered in its recipient's locale. */
            content: field.json(NotificationContent),
            /** The line a summary shows for it, such as "3 mentions", in its recipient's locale. */
            summary: field.string(),
            /** The actions offered beside it, the first the default. */
            actions: field.json(schema.array(ActionMetadata)),
            /** How strongly it interrupts. */
            interruption: field.enum(INTERRUPTION_LEVELS),
            /** The preference its recipient starts from. */
            preference: field.json(Preference),
        },
        permissions: {
            // the recipient reads it through its source
            read: intersection(relation("recipient"), through("parent", "read")),
            // the source's readers post what their calls do to it through their apps' code
            post: through("parent", "read"),
        },
        // expire an activity after 180 days, as code forges expire notifications after five months
        expiring: [{ after: { days: 180 }, from: "occurredAt" }],
        constraints: {
            key: {
                kind: "unique",
                on: [
                    "scope",
                    "recipientId",
                    "packageId",
                    "name",
                    "parentPackageId",
                    "parentType",
                    "parentId",
                    "key",
                ],
            },
            thread: { kind: "index", on: ["scope", "recipientId", "thread", "packageId", "name"] },
            count: { kind: "check", condition: { count: { gte: 1 } } },
        },
    },
    (method) => ({
        notify: method.mutation({
            permission: "post",
            target: "scope",
            isInternal: true,
            input: schema.object({
                /** The object it is about. */
                parent: ParentReference,
                /** The package declaring the notification. */
                packageId: PackageId,
                /** The notification's name in its package. */
                name: NotificationName,
                /** The principals it notifies. */
                recipients: schema.array(Subject).max(NOTIFY_RECIPIENTS),
                /** Why they receive it. */
                reason: schema.enum(REASONS),
                /** The values its content renders. */
                payload: schema.json(),
                /** The identity a later notice with the same key replaces. */
                key: NotificationKey.exactOptional(),
                /** The thread, the source's identifier by default. */
                thread: NotificationKey.exactOptional(),
            }),
            output: schema.object({}),
        }),
        post: method.create("post", { isInternal: true }),
        occur: method.update("post", { isInternal: true }),
        act: method.mutation({
            permission: "read",
            input: schema.object({
                /** The declared action's name. */
                action: ActionName,
                /** The text the action asks for. */
                text: schema.string().min(1).exactOptional(),
            }),
        }),
    }),
);

/** An activity as its table stores it. */
export type Activity = Select<typeof activity.table>;

/** The activities a person is notified of. */
export const Activity = {
    /** Read the person an activity is for. */
    recipient(row: Pick<Activity, keyof ReferenceColumns<"recipient">>): Subject {
        return ObjectReference.at("recipient", row);
    },

    /** Summarize activities, one line per declaration and thread: its latest summary line. */
    summarize(occurrences: readonly Activity[]): readonly string[] {
        const groups = new Map<string, Activity>();
        for (const occurrence of occurrences) {
            const group = `${occurrence.packageId} ${occurrence.name} ${occurrence.thread}`;
            const known = groups.get(group);
            if (known === undefined || known.occurredAt <= occurrence.occurredAt) {
                groups.set(group, occurrence);
            }
        }

        return [...groups.values()].map((latest) => latest.summary);
    },
};

/** The activities of one recipient a home replicates from a space. */
const ActivitySelection = schema.object({
    /** The recipient's identifier. */
    recipient: schema.string().min(1),
});

/** The shape replicating a person's activities from a space into their home, as the person. */
export const activityShape = defineShape({
    name: "activity",
    parameters: ActivitySelection,
    objects: [activity],
    queries: ({ recipient }) => [activity.query({ where: { recipientId: recipient } })],
});

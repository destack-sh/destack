import type { InstanceOf } from "@destack/object";
import { type ObjectReference, Subject } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { and, eq, gte, inArray, isNull } from "@destack/db";
import { Call, type ObjectType } from "@destack/object";
import type { Action, Content } from "./content.ts";
import type { Notice } from "./notice.ts";
import type { Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { schema, Duration } from "@destack/schema";
import { Setting } from "@destack/setting";
import { type Audience, announcement, type Announcement } from "../object/announcement.ts";
import { NOTIFY_RECIPIENTS, notification } from "../object/notification.ts";

import { type InterruptionLevel, Preference } from "../preference/preference.ts";

/** The largest payload as JSON, in bytes: half of a push message's 4096, beside content and encryption. */
const PAYLOAD_BYTES = 2048;

/** The default collapse window: a quarter hour, the default email delay. */
const COLLAPSE: Duration = { minutes: 15 };

/** A notification's declaration. */
export interface NotificationDefinition<Payload extends schema.Schema = schema.Schema> {
    /** The name, in camel case. */
    readonly name: string;
    /** The label settings show, such as "Mentions". */
    readonly title: string;
    /** What it tells. */
    readonly description: string;
    /** The payload schema. */
    readonly payload: Payload;
    /** How strongly it interrupts. */
    readonly interruption: InterruptionLevel;
    /** The preference recipients start from. */
    readonly preference: Preference;
    /** Render the text every channel shows. */
    content(payload: schema.Infer<Payload>): Content;
    /** Summarize a thread's notifications, such as "3 mentions". */
    summary(count: number): string;
    /** The actions, by name. */
    readonly actions?: Readonly<Record<string, Action<schema.Infer<Payload>>>>;
    /** How long an unread notification collapses later ones of its thread, 15 minutes by default. */
    readonly collapse?: Duration;
}

/** A declared notification. */
export class Notification<Payload extends schema.Schema = schema.Schema> {
    /** The declaring package. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The declaration. */
    readonly definition: NotificationDefinition<Payload>;
    /** The reference rows name. */
    readonly reference: DeclarationReference;
    /** The recipient's preference setting. */
    readonly preference: Setting<typeof Preference>;

    /** Keep a declaration and derive its preference setting. */
    constructor(owner: Package, definition: NotificationDefinition<Payload>) {
        // keep the declaration
        this.package = owner;
        this.name = definition.name;
        this.definition = definition;
        this.reference = DeclarationReference.of(this);

        // derive the preference setting
        this.preference = new Setting(owner, {
            name: `notification.${definition.name}`,
            title: definition.title,
            description: definition.description,
            schema: Preference,
            default: definition.preference,
            scope: "user",
            overrides: ["space", "installation", "device"],
            apply: "immediate",
        });
    }

    /** Serialise the notification as its reference. */
    toJSON(): DeclarationReference {
        return this.reference;
    }

    /** Report whether a row belongs to this declaration. */
    is(row: Pick<InstanceOf<typeof notification>, "packageId" | "name">): boolean {
        return row.packageId === this.reference.packageId && row.name === this.name;
    }

    /** Notify recipients inside a served call, never the caller. */
    async notify(
        call: Call,
        notice: Notice<schema.Infer<Payload>> & { readonly recipients: readonly Subject[] },
    ): Promise<readonly InstanceOf<typeof notification>[]> {
        // require a served call, a source, a small payload and few recipients
        call.served();
        this.#requireSource(call, notice.source, notification);
        const payload = this.#payload(notice.payload);
        if (notice.recipients.length > NOTIFY_RECIPIENTS) {
            throw new TypeError(`a notification names at most ${NOTIFY_RECIPIENTS} recipients`);
        }

        // name each recipient once, never the caller
        const recipients = [
            ...new Set(
                notice.recipients
                    .filter(
                        (recipient) =>
                            call.caller === undefined || !Subject.same(recipient, call.caller),
                    )
                    .map(Subject.key),
            ),
        ];
        if (recipients.length === 0) {
            return [];
        }

        // read the notifications the notice replaces or joins
        const table = notification.table;
        const { source } = notice;
        const thread = notice.thread ?? source.id;
        const current = await call.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.scope, source.scope as never),
                    inArray(table.recipient, recipients),
                    eq(table.packageId, this.reference.packageId),
                    eq(table.name, this.name),
                    notice.key === undefined
                        ? and(
                              eq(table.thread, thread),
                              isNull(table.readAt),
                              gte(
                                  table.createdAt,
                                  call.now -
                                      Duration.milliseconds(this.definition.collapse ?? COLLAPSE),
                              ),
                          )
                        : and(
                              eq(table.parentPackageId, source.packageId),
                              eq(table.parentType, source.type),
                              eq(table.parentId, source.id),
                              eq(table.key, notice.key),
                          ),
                ),
            )
            .orderBy(table.createdAt);
        const existing = new Map(current.map((row) => [row.recipient, row]));

        // replace or join each existing notification, and post the others
        const rows: InstanceOf<typeof notification>[] = [];
        for (const recipient of recipients) {
            const known = existing.get(recipient);
            const row =
                known === undefined
                    ? await call.invoke(notification, "post", {
                          parent: { packageId: source.packageId, type: source.type, id: source.id },
                          recipient,
                          packageId: this.reference.packageId,
                          name: this.name,
                          ...(notice.key === undefined ? {} : { key: notice.key }),
                          thread,
                          reason: notice.reason,
                          payload,
                          occurredAt: call.now,
                      })
                    : await this.#occur(call, known, notice, payload, thread);
            rows.push(row as InstanceOf<typeof notification>);
        }

        return rows;
    }

    /** Announce to an audience inside a served call, never to the caller. */
    async announce(
        call: Call,
        notice: Omit<Notice<schema.Infer<Payload>>, "reason"> & {
            /** The principals it reaches, and why. */
            readonly audience: Audience;
            /** The principals it skips beside the caller, such as those the call notified already. */
            readonly excluded?: readonly Subject[];
        },
    ): Promise<Announcement> {
        // require a served call, a source, a declared permission and a small payload
        this.#requireSource(call, notice.source, announcement);
        const { audience } = notice;
        const permissions = call.served().authorizer.policy(notice.source).definition.permissions;
        if (audience.kind === "permission" && !Object.hasOwn(permissions, audience.permission)) {
            throw new TypeError(
                `object ${notice.source.type} has no permission ${audience.permission}`,
            );
        }
        const payload = this.#payload(notice.payload);
        const author = call.caller;
        if (author === undefined) {
            throw new TypeError("an announcement excludes the principal whose call made it");
        }

        // read the announcement a keyed one replaces
        const table = announcement.table;
        const { source } = notice;
        const thread = notice.thread ?? source.id;
        const [known] =
            notice.key === undefined
                ? []
                : await call.database
                      .select()
                      .from(table)
                      .where(
                          and(
                              eq(table.packageId, this.reference.packageId),
                              eq(table.name, this.name),
                              eq(table.parentPackageId, source.packageId),
                              eq(table.parentType, source.type),
                              eq(table.parentId, source.id),
                              eq(table.key, notice.key),
                          ),
                      );
        const excluded = notice.excluded ?? [];
        if (excluded.length > NOTIFY_RECIPIENTS) {
            throw new TypeError(`an announcement skips at most ${NOTIFY_RECIPIENTS} principals`);
        }
        const values = {
            author: Subject.key(author),
            audience: notice.audience,
            excluded: [...new Set(excluded.map(Subject.key))],
            thread,
            payload,
            cursor: null,
            expandedAt: null,
        };

        // replace the existing announcement, expanding it again from the first entry
        if (known !== undefined) {
            return (await call.invoke(announcement, "replace", {
                id: known.id,
                ...values,
            })) as Announcement;
        }

        // post one otherwise
        return (await call.invoke(announcement, "post", {
            ...values,
            parent: { packageId: source.packageId, type: source.type, id: source.id },
            packageId: this.reference.packageId,
            name: this.name,
            ...(notice.key === undefined ? {} : { key: notice.key }),
        })) as Announcement;
    }

    /** Render a notification's text. */
    render(row: Pick<InstanceOf<typeof notification>, "payload">): Content {
        return this.definition.content(this.definition.payload.parse(row.payload));
    }

    /** Answer a notification with an action as its recipient, then read it. */
    async act(call: Call, action: string, text?: string): Promise<unknown> {
        // require a declared action and its text
        const declared = this.definition.actions?.[action];
        if (declared === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `notification ${this.name} has no action ${action}`,
            });
        } else if ((declared.text === undefined) !== (text === undefined)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `action ${action} of ${this.name} takes text only where it asks for it`,
            });
        }

        // make the action's effect and read the notification
        const row = call.target as InstanceOf<typeof notification>;
        const source: ObjectReference = {
            packageId: row.parentPackageId!,
            type: row.parentType!,
            scope: row.scope,
            id: row.parentId!,
        };
        const payload = this.definition.payload.parse(row.payload);
        await declared.effect({ source, payload }, call, text);

        return call.invoke(notification, "read", {
            [notification.route.field!]: row.scope,
            id: row.id,
        });
    }

    /** Replace or join an existing notification and leave it unread. */
    async #occur(
        call: Call,
        existing: InstanceOf<typeof notification>,
        notice: Notice<unknown>,
        payload: unknown,
        thread: string,
    ): Promise<unknown> {
        // replan replaced and snoozed notifications
        const isReplaced = notice.key !== undefined;
        const isPlanned = !isReplaced && existing.snoozedUntil === null;

        return call.invoke(notification, "occur", {
            id: existing.id,
            reason: notice.reason,
            payload,
            thread,
            count: isReplaced ? 1 : existing.count + 1,
            occurredAt: call.now,
            readAt: null,
            snoozedUntil: null,
            ...(isPlanned ? {} : { plannedAt: null }),
        });
    }

    /** Require a source in the call's scope that takes an attachment. */
    #requireSource(call: Call, source: ObjectReference, attachment: ObjectType): void {
        const type = call.objects.find((object) => object.policy.is(source));
        if (source.scope !== call.scope) {
            throw new TypeError(`notification ${this.name} has a source in another scope`);
        } else if (!type?.attachments.some((attached) => attachment.same(attached.object))) {
            throw new TypeError(`object ${source.type} takes no ${this.name} notifications`);
        }
    }

    /** Check a payload's schema and size. */
    #payload(value: unknown): schema.Infer<ReturnType<typeof schema.json>> {
        // parse and measure the payload
        const payload = schema.json().parse(this.definition.payload.parse(value));
        const bytes = new TextEncoder().encode(JSON.stringify(payload)).length;
        if (bytes > PAYLOAD_BYTES) {
            throw new TypeError(`notification ${this.name} carries at most ${PAYLOAD_BYTES} bytes`);
        }

        return payload;
    }
}

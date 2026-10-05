import { type ObjectReference, Subject } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { and, eq, gte, inArray, type SQL } from "@destack/db";
import { Call, type CallOf, type ObjectType, type ResultOf } from "@destack/object";
import { type Action, ActionMetadata, ContentDefinition } from "./content.ts";
import type { Notice } from "./notice.ts";
import { Catalog, type LocaleTag, Localization, type Message } from "@destack/locale";
import type { Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { Duration, type JsonValue, schema, zip } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { activity, type Activity, NOTIFY_RECIPIENTS } from "../object/activity.ts";
import { type Audience, announcement, type Announcement } from "../object/announcement.ts";
import { type InterruptionLevel, Preference } from "../preference/preference.ts";
import { preferenceSetting } from "../setting/setting.ts";

/** The largest payload as JSON, in bytes: half of a push message's 4096, beside content and encryption. */
const PAYLOAD_BYTES = 2048;

/** The default collapse window: a quarter hour, the default email delay. */
const COLLAPSE: Duration = { minutes: 15 };

/** The language of a recipient who set none: the source language declarations write their messages in. */
const SOURCE_LOCALE: LocaleTag = "en";

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
    /** Write the text every channel shows, its messages rendered in each recipient's locale. */
    content(payload: schema.Infer<Payload>): ContentDefinition;
    /** Summarize a thread's notifications as a plural message, such as t`${plural(count, { one: "# mention", other: "# mentions" })}`. */
    summary(count: number): Message;
    /** The actions, by name. */
    readonly actions?: Readonly<Record<string, Action<schema.Infer<Payload>>>>;
    /** How long an unread notification collapses later ones of its thread, 15 minutes by default. */
    readonly collapse?: Duration;
}

/** A declared notification. */
export class NotificationType<Payload extends schema.Schema = schema.Schema> {
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
        this.preference = preferenceSetting(owner, definition);
    }

    /** Serialise the notification as its reference. */
    toJSON(): DeclarationReference {
        return this.reference;
    }

    /** Report whether a row belongs to this declaration. */
    is(row: Pick<Activity, "packageId" | "name">): boolean {
        return row.packageId === this.reference.packageId && row.name === this.name;
    }

    /** Record an activity for each recipient inside a served call, never for the caller. */
    async notify(
        call: Call,
        notice: Notice<schema.Infer<Payload>> & { readonly recipients: readonly Subject[] },
    ): Promise<readonly Activity[]> {
        // require a served call, a source, a small payload and few recipients
        call.requireAuthorization();
        this.#requireSource(call, notice.source, activity);
        const payload = this.#payload(notice.payload);
        if (notice.recipients.length > NOTIFY_RECIPIENTS) {
            throw new TypeError(`a notification names at most ${NOTIFY_RECIPIENTS} recipients`);
        }

        // name each recipient once, never the caller
        const recipients = NotificationType.#recipients(call, notice.recipients);
        if (recipients.length === 0) {
            return [];
        }

        // replace or join each existing activity, and post the others, rendered in each recipient's locale
        const thread = notice.thread ?? notice.source.id;
        const existing = await this.#existing(call, notice, recipients, thread);
        const locales = await NotificationType.#localizations(call, recipients);
        const rows: Activity[] = [];
        for (const [recipient, locale] of locales) {
            const known = existing.get(recipient);
            if (known === undefined) {
                rows.push(await this.#post(call, notice, { recipient, locale }, payload, thread));
            } else {
                rows.push(await this.#occur(call, known, notice, { payload, locale }, thread));
            }
        }

        return rows;
    }

    /** Announce to an audience inside a served call, never to its caller, as the system without one. */
    async announce(
        call: Call,
        notice: Omit<Notice<schema.Infer<Payload>>, "reason"> & {
            /** The principals it notifies, and why. */
            readonly audience: Audience;
            /** The principals it skips beside the caller, such as those the call notified already. */
            readonly excluded?: readonly Subject[];
        },
    ): Promise<Announcement> {
        // require a served call on a source with a declared audience
        this.#requireSource(call, notice.source, announcement);
        NotificationType.#requireAudience(call, notice.source, notice.audience);

        // require a small payload
        const payload = this.#payload(notice.payload);

        // require few exclusions
        const excluded = notice.excluded ?? [];
        if (excluded.length > NOTIFY_RECIPIENTS) {
            throw new TypeError(`an announcement skips at most ${NOTIFY_RECIPIENTS} principals`);
        }

        // replace the announcement a keyed one replaces, expanding it again from the first entry
        const { source } = notice;
        const values = {
            author: call.caller === undefined ? null : Subject.key(call.caller),
            audience: notice.audience,
            excluded: [...new Set(excluded.map((principal) => Subject.key(principal)))],
            thread: notice.thread ?? source.id,
            payload,
            cursor: null,
            expandedAt: null,
        };
        const known =
            notice.key === undefined ? undefined : await this.#announced(call, source, notice.key);
        if (known !== undefined) {
            return call.invoke(announcement).replace({ id: known.id, ...values });
        }

        // post one otherwise
        return call.invoke(announcement).post({
            ...values,
            parent: { packageId: source.packageId, type: source.type, id: source.id },
            packageId: this.reference.packageId,
            name: this.name,
            ...(notice.key === undefined ? {} : { key: notice.key }),
        });
    }

    /** Answer an activity with an action as its recipient. */
    async act(
        call: CallOf<typeof activity, "act">,
        action: string,
        text?: string,
    ): Promise<ResultOf<typeof activity, "act">> {
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

        // make the action's effect
        const row = call.target;
        const source: ObjectReference = {
            packageId: row.parentPackageId,
            type: row.parentType,
            scope: row.scope,
            id: row.parentId,
        };
        const payload = this.definition.payload.parse(row.payload);
        await declared.effect({ source, payload }, call, text);

        return row;
    }

    /** Name each recipient once by subject key, leaving out the caller. */
    static #recipients(call: Call, recipients: readonly Subject[]): string[] {
        const others = recipients.filter(
            (recipient) => call.caller === undefined || !Subject.same(recipient, call.caller),
        );

        return [...new Set(others.map((recipient) => Subject.key(recipient)))];
    }

    /** Read the activities a notice replaces by its key, or joins in its thread within the collapse window, by recipient. */
    async #existing(
        call: Call,
        notice: Notice<unknown>,
        recipients: readonly string[],
        thread: string,
    ): Promise<ReadonlyMap<string, Activity>> {
        // match the source's keyed activity, or the thread's recent ones
        const table = activity.table;
        const { source } = notice;
        let matched: SQL | undefined;
        if (notice.key === undefined) {
            const since = call.now - Duration.milliseconds(this.definition.collapse ?? COLLAPSE);
            matched = and(eq(table.thread, thread), gte(table.createdAt, since));
        } else {
            matched = and(
                eq(table.parentPackageId, source.packageId),
                eq(table.parentType, source.type),
                eq(table.parentId, source.id),
                eq(table.key, notice.key),
            );
        }

        // read them, the latest of each recipient last
        const current = await call.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.scope, schema.identifier("space").parse(source.scope)),
                    inArray(table.recipient, [...recipients]),
                    eq(table.packageId, this.reference.packageId),
                    eq(table.name, this.name),
                    matched,
                ),
            )
            .orderBy(table.createdAt);

        return new Map(current.map((row) => [row.recipient, row]));
    }

    /** Read each recipient's localization with the catalogs the installation's build ships, the declaring package's own. */
    static async #localizations(
        call: Call,
        recipients: readonly string[],
    ): Promise<ReadonlyMap<string, Localization>> {
        // require the installation serving the call
        const { installation } = call;
        if (installation === undefined) {
            throw new TypeError("notifications render in their recipients' locales inside a space");
        }

        // read the build's catalogs and each recipient's locale
        const catalogs = await Catalog.read(installation.build);
        const tags = await Promise.all(
            recipients.map((recipient) => installation.directory.locale(recipient)),
        );

        return new Map(
            zip(recipients, tags).map(([recipient, tag]) => [
                recipient,
                Localization.of(tag ?? SOURCE_LOCALE, catalogs),
            ]),
        );
    }

    /** Post a recipient's first activity of a notice, rendered in their locale. */
    #post(
        call: Call,
        notice: Notice<unknown>,
        to: { readonly recipient: string; readonly locale: Localization },
        payload: JsonValue,
        thread: string,
    ): Promise<Activity> {
        const { source } = notice;
        const { recipient, locale } = to;

        return call.invoke(activity).post({
            parent: { packageId: source.packageId, type: source.type, id: source.id },
            recipient,
            ...(call.caller === undefined ? {} : { actor: Subject.key(call.caller) }),
            packageId: this.reference.packageId,
            release: this.package,
            name: this.name,
            ...(notice.key === undefined ? {} : { key: notice.key }),
            thread,
            reason: notice.reason,
            payload,
            occurredAt: call.now,
            ...this.#rendering(payload, 1, locale),
        });
    }

    /** Refuse an audience of a permission the source's object type does not declare. */
    static #requireAudience(call: Call, source: ObjectReference, audience: Audience): void {
        const permissions = call.requireAuthorization().authorizer.policy(source)
            .definition.permissions;
        if (audience.kind === "permission" && !Object.hasOwn(permissions, audience.permission)) {
            throw new TypeError(`object ${source.type} has no permission ${audience.permission}`);
        }
    }

    /** Read the announcement of a source under a key, absent before one. */
    async #announced(
        call: Call,
        source: ObjectReference,
        key: string,
    ): Promise<Announcement | undefined> {
        const table = announcement.table;
        const [known] = await call.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.packageId, this.reference.packageId),
                    eq(table.name, this.name),
                    eq(table.parentPackageId, source.packageId),
                    eq(table.parentType, source.type),
                    eq(table.parentId, source.id),
                    eq(table.key, key),
                ),
            );

        return known;
    }

    /** Replace or join an existing activity, counting the occurrence, rendered in its recipient's locale. */
    async #occur(
        call: Call,
        existing: Activity,
        notice: Notice<unknown>,
        rendered: { readonly payload: JsonValue; readonly locale: Localization },
        thread: string,
    ): Promise<ResultOf<typeof activity, "occur">> {
        const { payload, locale } = rendered;
        const count = notice.key === undefined ? existing.count + 1 : 1;

        return call.invoke(activity).occur({
            id: existing.id,
            reason: notice.reason,
            payload,
            thread,
            count,
            occurredAt: call.now,
            ...this.#rendering(payload, count, locale),
        });
    }

    /** Render an activity's text and summary line in its recipient's locale, with its actions, level and preference. */
    #rendering(payload: JsonValue, count: number, locale: Localization) {
        const { definition } = this;
        const actions = Object.entries(definition.actions ?? {}).map(
            ([name, action]): [string, ActionMetadata] => [
                name,
                ActionMetadata.parse(
                    schema.defined({
                        title: action.title,
                        isDestructive: action.isDestructive,
                        text: action.text,
                    }),
                ),
            ],
        );

        return {
            content: ContentDefinition.render(
                definition.content(definition.payload.parse(payload)),
                locale,
            ),
            summary: locale.render(definition.summary(count)),
            actions: Object.fromEntries(actions),
            interruption: definition.interruption,
            preference: definition.preference,
        };
    }

    /** Require a source in the call's scope that takes an attachment. */
    #requireSource(call: Call, source: ObjectReference, attachment: ObjectType): void {
        const type = call.objects.find((object) => object.policy.is(source));
        if (source.scope !== call.scope) {
            throw new TypeError(`notification ${this.name} has a source in another scope`);
        } else if (
            type === undefined ||
            !type.attachments.some((attached) => attachment.same(attached.object))
        ) {
            throw new TypeError(`object ${source.type} takes no ${this.name} notifications`);
        }
    }

    /** Check a payload's schema and size. */
    #payload(value: unknown): JsonValue {
        // parse and measure the payload
        const payload = schema.json().parse(this.definition.payload.parse(value));
        const bytes = new TextEncoder().encode(JSON.stringify(payload)).length;
        if (bytes > PAYLOAD_BYTES) {
            throw new TypeError(`notification ${this.name} takes at most ${PAYLOAD_BYTES} bytes`);
        }

        return payload;
    }
}

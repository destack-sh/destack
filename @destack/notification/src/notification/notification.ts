import { Catalog, Localization, type Message } from "@destack/locale";
import type { Call, CallOf, ResultOf } from "@destack/object";
import type { Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { BuildCache } from "@destack/package/manifest";
import { Duration, type JsonValue, schema, zip } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Setting } from "@destack/setting";
import { ObjectReference, Subject } from "@destack/sync";
import { activity, type Activity } from "../object/activity.ts";
import { type Audience, announcement, type Announcement } from "../object/announcement.ts";
import { type InterruptionLevel, Preference } from "../preference/preference.ts";
import { preferenceSetting } from "../setting/setting.ts";
import { type Action, ActionMetadata, WordingDefinition } from "./content.ts";
import type { Notice } from "./notice.ts";

/** The largest payload as JSON, in bytes. */
const PAYLOAD_BYTES = 2048;

/** The default collapse window: a quarter hour, the default email delay. */
const COLLAPSE: Duration = { minutes: 15 };

/** The catalogs read from each installation's build so far. */
const CATALOGS = new BuildCache((reader) => Catalog.read(reader));

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
    content(payload: schema.Infer<Payload>): WordingDefinition;
    /** Summarize a thread's notifications as a plural message. */
    summary(count: number): Message;
    /** The actions, by name. */
    readonly actions?: Readonly<Record<string, Action<schema.Infer<Payload>>>>;
    /** How long an unread notification collapses later ones of its thread. */
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
    /** The reference rows carry. */
    readonly reference: DeclarationReference;
    /** The recipient's preference setting. */
    readonly preference: Setting<typeof Preference>;
    /** The metadata of each action by name: what its button shows. */
    readonly actions: Readonly<Record<string, ActionMetadata>>;

    /** Keep a declaration and derive its preference setting and action metadata. */
    constructor(owner: Package, definition: NotificationDefinition<Payload>) {
        // keep the declaration
        this.package = owner;
        this.name = definition.name;
        this.definition = definition;
        this.reference = DeclarationReference.of(this);

        // derive the preference setting and the action metadata
        this.preference = preferenceSetting(owner, definition);
        this.actions = Object.fromEntries(
            Object.entries(definition.actions ?? {}).map(([name, action]) => [
                name,
                ActionMetadata.parse(
                    schema.defined({
                        title: action.title,
                        isDestructive: action.isDestructive,
                        text: action.text,
                    }),
                ),
            ]),
        );
    }

    /** Serialise the notification as its reference. */
    toJSON(): DeclarationReference {
        return this.reference;
    }

    /** Report whether a row belongs to this declaration. */
    is(row: Pick<Activity, "packageId" | "name">): boolean {
        return row.packageId === this.reference.packageId && row.name === this.name;
    }

    /** Build the call notifying a notice's named recipients or its audience. */
    call(notice: Notice<schema.Infer<Payload>>) {
        // refer to the source as the parent in the call's scope, with the declaration and payload
        const { source, key, thread } = notice;
        const fields = {
            parent: { packageId: source.packageId, type: source.type, id: source.id },
            packageId: this.reference.packageId,
            name: this.name,
            payload: this.#payload(notice.payload),
            ...(key === undefined ? {} : { key }),
            ...(thread === undefined ? {} : { thread }),
        };

        // record an activity for each named recipient
        if ("recipients" in notice) {
            const { recipients, reason } = notice;

            return activity.call("notify", { ...fields, recipients: [...recipients], reason });
        }
        // announce to an audience
        else {
            const { audience, excluded } = notice;

            return announcement.call("announce", {
                ...fields,
                audience,
                ...(excluded === undefined ? {} : { excluded: [...excluded] }),
            });
        }
    }

    /** Record an activity for each named recipient, replacing or joining their existing one. */
    async notify(
        call: CallOf<typeof activity, "notify">,
    ): Promise<ResultOf<typeof activity, "notify">> {
        // require a small payload and recipients other than the caller, each listed once
        const payload = this.#payload(call.input.payload);
        const recipients = NotificationType.#recipients(call, call.input.recipients);
        if (recipients.length === 0) {
            return {};
        }

        // read the existing activities and each recipient's locale
        const thread = call.input.thread ?? call.input.parent.id;
        const existing = await this.#existing(call, recipients, thread);
        const locales = await NotificationType.#localizations(call, recipients);

        // replace or join each existing activity, and post the others in one mutation
        const calls = [...locales].map(([recipient, locale]) => {
            const known = existing.get(recipient);

            return known === undefined
                ? this.#post(call, recipient, { payload, locale, thread })
                : this.#occur(call, known, { payload, locale, thread });
        });
        await call.mutate(call.caller, calls);

        return {};
    }

    /** Post or replace the announcement of a notice to an audience. */
    async announce(
        call: CallOf<typeof announcement, "announce">,
    ): Promise<ResultOf<typeof announcement, "announce">> {
        // require a small payload and an audience the source's type declares
        const { parent, audience, key } = call.input;
        const payload = this.#payload(call.input.payload);
        const source: ObjectReference = { ...parent, scope: call.scope };
        NotificationType.#requireAudience(call, source, audience);

        // read the announcement a keyed one replaces
        const { subject } = call.caller;
        const excluded = (call.input.excluded ?? []).map((principal) => Subject.key(principal));
        const values = {
            author: subject === undefined ? null : Subject.key(subject),
            audience,
            excluded: [...new Set(excluded)],
            thread: call.input.thread ?? parent.id,
            payload,
            cursor: null,
            expandedAt: null,
        };
        const known = key === undefined ? undefined : await this.#announced(call, source, key);

        // replace it
        if (known !== undefined) {
            await call.mutate(call.caller, [
                announcement.call("replace", { id: known.id, ...values }),
            ]);
        }
        // post one otherwise
        else {
            await call.mutate(call.caller, [
                announcement.call("post", {
                    ...values,
                    parent,
                    packageId: this.reference.packageId,
                    name: this.name,
                    ...(key === undefined ? {} : { key }),
                }),
            ]);
        }

        return {};
    }

    /** Answer an activity with an action as its recipient. */
    async act(
        call: CallOf<typeof activity, "act">,
        action: string,
        text?: string,
    ): Promise<ResultOf<typeof activity, "act">> {
        // require a declared action
        const declared = this.definition.actions?.[action];
        if (declared === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `notification ${this.name} has no action ${action}`,
            });
        }
        // require text exactly where the action asks for it
        else if ((declared.text === undefined) !== (text === undefined)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `action ${action} of ${this.name} takes text only where it asks for it`,
            });
        }

        // make the action's effect
        const row = call.target;
        const payload = this.definition.payload.parse(row.payload);
        await declared.effect({ source: ObjectReference.parent(row), payload }, call, text);

        return row;
    }

    /** Parse a payload and require its size within the limit. */
    #payload(value: unknown): JsonValue {
        // parse and measure the payload
        const payload = schema.json().parse(this.definition.payload.parse(value));
        const bytes = new TextEncoder().encode(JSON.stringify(payload)).length;
        if (bytes > PAYLOAD_BYTES) {
            throw new TypeError(`notification ${this.name} takes at most ${PAYLOAD_BYTES} bytes`);
        }

        return payload;
    }

    /** Refuse an audience of a permission the source's object type does not declare. */
    static #requireAudience(call: Call, source: ObjectReference, audience: Audience): void {
        const { permissions } = call.requireAuthorization().authorizer.policy(source).definition;
        if (audience.kind === "permission" && !Object.hasOwn(permissions, audience.permission)) {
            throw new TypeError(`object ${source.type} has no permission ${audience.permission}`);
        }
    }

    /** Key each recipient once by subject key, leaving out the caller. */
    static #recipients(call: Call, recipients: readonly Subject[]): string[] {
        const { subject } = call.caller;
        const others = recipients.filter(
            (recipient) => subject === undefined || !Subject.same(recipient, subject),
        );

        return [...new Set(others.map((recipient) => Subject.key(recipient)))];
    }

    /** Read the activities a notice replaces by its key, or joins in its thread, by recipient. */
    async #existing(
        call: CallOf<typeof activity, "notify">,
        recipients: readonly string[],
        thread: string,
    ): Promise<ReadonlyMap<string, Activity>> {
        // match the source's keyed activity, or the thread's recent ones
        const { parent, key } = call.input;
        const since = call.now - Duration.milliseconds(this.definition.collapse ?? COLLAPSE);
        const matched =
            key === undefined
                ? { thread, createdAt: { gte: since } }
                : {
                      parentPackageId: parent.packageId,
                      parentType: parent.type,
                      parentId: parent.id,
                      key,
                  };

        // read them
        const current = await call.read(activity).rows({
            scope: schema.identifier("space").parse(call.scope),
            recipient: { in: [...recipients] },
            packageId: this.reference.packageId,
            name: this.name,
            ...matched,
        });

        // sort by creation so the latest of each recipient wins
        const ordered = current.toSorted((left, right) => left.createdAt - right.createdAt);

        return new Map(ordered.map((row) => [row.recipient, row]));
    }

    /** Read each recipient's localization with the catalogs the installation's build ships. */
    static async #localizations(
        call: Call,
        recipients: readonly string[],
    ): Promise<ReadonlyMap<string, Localization>> {
        // read the build's catalogs and each recipient's locale
        const installation = call.requireInstallation();
        const catalogs = await CATALOGS.read(installation.build);
        const tags = await Promise.all(
            recipients.map((recipient) => installation.directory.locale(recipient)),
        );

        return new Map(
            zip(recipients, tags).map(([recipient, tag]) => [
                recipient,
                Localization.of(tag, catalogs),
            ]),
        );
    }

    /** Build the call posting a recipient's first activity of a notice. */
    #post(call: CallOf<typeof activity, "notify">, recipient: string, occurrence: Occurrence) {
        const { parent, key, reason } = call.input;
        const { subject } = call.caller;

        return activity.call("post", {
            parent,
            recipient,
            ...(subject === undefined ? {} : { actor: Subject.key(subject) }),
            packageId: this.reference.packageId,
            release: this.package,
            name: this.name,
            ...(key === undefined ? {} : { key }),
            thread: occurrence.thread,
            reason,
            payload: occurrence.payload,
            occurredAt: call.now,
            ...this.#rendering(occurrence, 1),
        });
    }

    /** Build the call replacing or joining an existing activity, counting the occurrence. */
    #occur(call: CallOf<typeof activity, "notify">, existing: Activity, occurrence: Occurrence) {
        const count = call.input.key === undefined ? existing.count + 1 : 1;

        return activity.call("occur", {
            id: existing.id,
            reason: call.input.reason,
            payload: occurrence.payload,
            thread: occurrence.thread,
            count,
            occurredAt: call.now,
            ...this.#rendering(occurrence, count),
        });
    }

    /** Render an activity's fields in its recipient's locale. */
    #rendering(occurrence: Occurrence, count: number) {
        // render the content and the summary line in the recipient's locale
        const { definition } = this;
        const { payload, locale } = occurrence;
        const content = definition.content(definition.payload.parse(payload));

        return {
            content: WordingDefinition.render(content, locale),
            summary: locale.render(definition.summary(count)),
            actions: this.actions,
            interruption: definition.interruption,
            preference: definition.preference,
        };
    }

    /** Read the announcement of a source under a key, absent before one. */
    async #announced(
        call: Call,
        source: ObjectReference,
        key: string,
    ): Promise<Announcement | undefined> {
        const [known] = await call.read(announcement).rows({
            packageId: this.reference.packageId,
            name: this.name,
            parentPackageId: source.packageId,
            parentType: source.type,
            parentId: source.id,
            key,
        });

        return known;
    }
}

/** One occurrence of a notice for one recipient. */
interface Occurrence {
    /** The parsed payload. */
    readonly payload: JsonValue;
    /** The recipient's localization. */
    readonly locale: Localization;
    /** The thread it groups in. */
    readonly thread: string;
}

import { type Localization, Message } from "@destack/locale";
import {
    MethodContext,
    type MethodContextOf,
    type ObjectType,
    type ResultOf,
} from "@destack/object";
import { Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { Duration, JsonValue, schema, toJsonSchema, zip } from "@destack/schema";
import { NotificationError } from "../error/error.ts";
import { ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import { Activity, activity, NotificationName } from "../object/activity.ts";
import { announcement, type Announcement } from "../object/announcement.ts";
import {
    INTERRUPTION_LEVELS,
    type InterruptionLevel,
    Preference,
} from "../preference/preference.ts";
import { Action, type ActionDefinition, NotificationContentDefinition } from "./content.ts";
import type { Notice } from "./notice.ts";

/** The largest payload as JSON, in bytes. */
const PAYLOAD_BYTES = 2048;

/** The default collapse window: a quarter hour, the default email delay. */
const COLLAPSE: Duration = { minutes: 15 };

/** A notification's metadata. */
export const NotificationMetadata = schema.object({
    /** The name, in camel case. */
    name: NotificationName,
    /** The label settings show. */
    title: schema.string().min(1),
    /** What it tells. */
    description: schema.string().min(1),
    /** How strongly it interrupts. */
    interruption: schema.enum(INTERRUPTION_LEVELS),
    /** The preference recipients start from. */
    preference: Preference,
});

/** A notification as manifests describe it. */
export const NotificationDescription = NotificationMetadata.extend({
    /** The declaring package. */
    package: Package,
    /** The payload's JSON Schema. */
    payload: schema.jsonObject(),
    /** The content every channel shows, its arguments the payload's fields. */
    content: NotificationContentDefinition,
    /** The line summarizing a thread's notifications, its argument the count. */
    summary: Message,
    /** The actions, the first the default. */
    actions: schema.array(Action),
    /** How long after its first occurrence a notification collapses later ones of its thread. */
    collapse: Duration.schema.exactOptional(),
});
/** A declared notification as manifests describe it. */
export type NotificationDescription = schema.Infer<typeof NotificationDescription>;

/** The payload fields a notification's messages take as arguments. */
export type PayloadField<Payload extends schema.Schema> = keyof schema.Infer<Payload> & string;

/** A notification's declaration: typed data its build describes. */
export interface NotificationDefinition<
    Payload extends schema.Schema = schema.Schema,
    Types extends readonly ObjectType[] = readonly ObjectType[],
> {
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
    /** The content every channel shows, its arguments the payload's fields. */
    readonly content: NotificationContentDefinition<PayloadField<Payload>>;
    /** The line summarizing a thread's notifications, its argument the count. */
    readonly summary: Message<"count">;
    /** The actions beside it, the first the default. */
    readonly actions?: {
        readonly [Index in keyof Types]: ActionDefinition<schema.Infer<Payload>, Types[Index]>;
    };
    /** How long after its first occurrence a notification collapses later ones of its thread. */
    readonly collapse?: Duration;
}

/** A declared notification. */
export class NotificationType<Payload extends schema.Schema = schema.Schema> {
    /** The declaring package's release. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The declaration beside its actions. */
    readonly definition: Omit<NotificationDefinition<Payload>, "actions">;
    /** The actions, the first the default. */
    readonly actions: readonly Action[];
    /** The declaration's reference, as its rows store it. */
    readonly reference: DeclarationReference;

    /** Create a declared type and derive its preference setting. */
    constructor(
        owner: Package,
        definition: Omit<NotificationDefinition<Payload>, "actions">,
        actions: readonly Action[],
    ) {
        // store the declaration and derive its reference and preference setting
        this.package = owner;
        this.name = definition.name;
        this.definition = definition;
        this.actions = actions;
        this.reference = DeclarationReference.of(this);
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
            payload: this.payload(notice.payload),
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

    /** Describe the notification for manifests. */
    describe(): NotificationDescription {
        const { definition, actions } = this;
        const { name, title, description, interruption, preference, content, summary } = definition;

        return {
            name,
            title,
            description,
            interruption,
            preference,
            package: this.package,
            payload: JsonValue.object(toJsonSchema(definition.payload)),
            content,
            summary,
            actions: [...actions],
            ...(definition.collapse === undefined ? {} : { collapse: definition.collapse }),
        };
    }

    /** List the notification's term: its name, with its payload's shape. */
    vocabulary(): Record<string, JsonValue> {
        const { name, payload } = this.definition;

        return { [name]: { payload: JsonValue.object(toJsonSchema(payload)) } };
    }

    /** Render an activity's fields in its recipient's locale, counting its occurrences. */
    render(payload: JsonValue, count: number, locale: Localization) {
        // fill the messages' arguments from the payload, and the summary's from the count
        const { definition } = this;
        const values = schema.record(schema.string(), schema.json()).parse(payload);
        const { summary } = definition;

        return {
            content: NotificationContentDefinition.render(definition.content, values, locale),
            summary: locale.render(summary, { count }),
            actions: this.actions.map(({ name, title, isDestructive, text }) => ({
                name,
                title,
                ...(isDestructive === undefined ? {} : { isDestructive }),
                ...(text === undefined ? {} : { text }),
            })),
            interruption: definition.interruption,
            preference: definition.preference,
        };
    }

    /** Parse a payload and require its size within the limit. */
    payload(value: unknown): JsonValue {
        // parse and measure the payload
        const payload = schema.json().parse(this.definition.payload.parse(value));
        const bytes = new TextEncoder().encode(JSON.stringify(payload)).length;
        if (bytes > PAYLOAD_BYTES) {
            throw NotificationError.of(
                "PAYLOAD_TOO_LARGE",
                `notification ${this.name} exceeds ${PAYLOAD_BYTES} bytes`,
            );
        }

        return payload;
    }

    /** Record an activity for each named recipient, replacing or joining their existing one. */
    async notify(
        localize: (recipient: Subject) => Promise<Localization>,
        context: MethodContext,
        notice: NotifyInput,
        at: { readonly occurredAt: number; readonly actor: Subject },
    ): Promise<ResultOf<typeof activity, "notify">> {
        // require a small payload and recipients other than the caller, each listed once
        const payload = this.payload(notice.payload);
        const recipients = NotificationType.#recipients(context, notice.recipients);
        if (recipients.length === 0) {
            return {};
        }

        // read the existing activities and each recipient's localization
        const { occurredAt, actor } = at;
        const thread = notice.thread ?? notice.parent.id;
        const existing = await this.#existing(context, notice, recipients, { thread, occurredAt });
        const locales = await Promise.all(recipients.map((recipient) => localize(recipient)));

        // replace or join each existing activity, and post the others in one mutation
        const calls = zip(recipients, locales).map(([recipient, locale]) => {
            const known = existing.get(Subject.key(recipient));

            const occurrence = { payload, locale, thread, occurredAt, actor };

            return known === undefined
                ? this.#post(notice, recipient, occurrence)
                : this.#occur(notice, known, occurrence);
        });
        await context.mutate(context.caller, calls);

        return {};
    }

    /** Post or replace the announcement of a notice to an audience. */
    async announce(
        context: MethodContextOf<typeof announcement, "announce">,
    ): Promise<ResultOf<typeof announcement, "announce">> {
        // require a small payload
        const { parent, audience, key } = context.input;
        const payload = this.payload(context.input.payload);
        const source: ObjectReference = { ...parent, scope: context.scope };

        // read the announcement a keyed one replaces
        const { subject } = context.caller;
        const excluded = (context.input.excluded ?? []).map((each) => Subject.key(each));
        const values = {
            ...ObjectReference.columns("author", subject),
            audience,
            excluded: [...new Set(excluded)],
            thread: context.input.thread ?? parent.id,
            payload,
            postedAt: context.now,
            cursor: null,
            expandedAt: null,
        };
        const known = key === undefined ? undefined : await this.#announced(context, source, key);

        // replace it
        if (known !== undefined) {
            await context.mutate(context.caller, [
                announcement.call("replace", { id: known.id, ...values }),
            ]);
        }
        // post one otherwise
        else {
            await context.mutate(context.caller, [
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

    /** Answer an activity with an action, sending its call for the recipient after the commit. */
    async act(
        name: string,
        text: string | undefined,
        context: MethodContextOf<typeof activity, "act">,
    ): Promise<ResultOf<typeof activity, "act">> {
        // require a declared action
        const declared = this.actions.find((action) => action.name === name);
        if (declared === undefined) {
            throw NotificationError.of(
                "ACTION_UNKNOWN",
                `notification ${this.name} has no action ${name}`,
            );
        }
        // require text exactly where the action asks for it
        else if ((declared.text === undefined) !== (text === undefined)) {
            throw NotificationError.of(
                "TEXT_UNEXPECTED",
                `action ${name} of ${this.name} must take text exactly when it declares text`,
            );
        }

        // find the type the action calls among the served ones
        const row = context.target;
        const key = ObjectTypeReference.key(declared.type);
        const called = context.objects.find(
            (object) => ObjectTypeReference.key(object.typeReference) === key,
        );
        if (called === undefined) {
            throw NotificationError.of(
                "ACTION_TYPE_NOT_SERVED",
                `action ${name} of ${this.name} calls ${key}, which this space serves no more`,
            );
        }

        // make the call as the person acting, on the target the payload identifies, else on the source
        const id = NotificationType.#target(row, declared);
        const input = {
            ...called.target({ scope: row.scope, id }),
            ...(text === undefined ? {} : { text }),
        };
        await context.mutate(context.caller, [called.call(declared.method, input)]);

        return row;
    }

    /** Read the identifier of an action's target: a payload field, else the activity's source. */
    static #target(row: Activity, action: Action): string {
        // read the target the payload identifies
        if (action.target !== undefined) {
            const payload = schema.jsonObject().parse(row.payload);

            return schema.string().parse(payload[action.target]);
        }
        // require the source to be of the type the action calls
        else if (
            row.parentPackageId !== action.type.packageId ||
            row.parentType !== action.type.type
        ) {
            throw new TypeError(
                `action ${action.name} calls ${action.type.type}, not the source ${row.parentType}`,
            );
        }

        return row.parentId;
    }

    /** List each recipient once, leaving out the caller. */
    static #recipients(context: MethodContext, recipients: readonly Subject[]): Subject[] {
        const { subject } = context.caller;
        const others = recipients.filter(
            (recipient) => subject === undefined || !Subject.same(recipient, subject),
        );

        return [
            ...new Map(others.map((recipient) => [Subject.key(recipient), recipient])).values(),
        ];
    }

    /** Read the activities a notice replaces by its key, or joins in its thread, by recipient. */
    async #existing(
        context: MethodContext,
        notice: NotifyInput,
        recipients: readonly Subject[],
        occurrence: { readonly thread: string; readonly occurredAt: number },
    ): Promise<ReadonlyMap<string, Activity>> {
        // match the source's keyed activity, or the thread's bursts started within the window
        const { parent, key } = notice;
        const { thread, occurredAt } = occurrence;
        const since = occurredAt - Duration.milliseconds(this.definition.collapse ?? COLLAPSE);
        const matched =
            key === undefined
                ? { thread, key: { isNull: true } as const, firstOccurredAt: { gte: since } }
                : {
                      parentPackageId: parent.packageId,
                      parentType: parent.type,
                      parentId: parent.id,
                      key,
                  };

        // read them
        const current = await context.read(activity).findMany({
            where: {
                scope: schema.identifier("space").parse(context.scope),
                recipientId: { in: recipients.map((recipient) => recipient.id) },
                packageId: this.reference.packageId,
                name: this.name,
                ...matched,
            },
        });

        // sort by creation so the latest of each recipient wins
        const ordered = current.toSorted((left, right) => left.createdAt - right.createdAt);

        return new Map(ordered.map((row) => [Subject.key(Activity.recipient(row)), row]));
    }

    /** Build the call posting a recipient's first activity of a notice, by its actor. */
    #post(notice: NotifyInput, recipient: Subject, occurrence: Occurrence) {
        const { parent, key, reason } = notice;

        return activity.call("post", {
            parent,
            ...ObjectReference.columns("recipient", recipient),
            ...ObjectReference.columns("actor", occurrence.actor),
            packageId: this.reference.packageId,
            name: this.name,
            ...(key === undefined ? {} : { key }),
            thread: occurrence.thread,
            reason,
            payload: occurrence.payload,
            firstOccurredAt: occurrence.occurredAt,
            occurredAt: occurrence.occurredAt,
            ...this.render(occurrence.payload, 1, occurrence.locale),
        });
    }

    /** Build the call replacing or joining an existing activity, counting the occurrence. */
    #occur(notice: NotifyInput, known: Activity, occurrence: Occurrence) {
        // replace a keyed activity as a fresh occurrence, or join the thread's burst
        const isReplaced = notice.key !== undefined;
        const count = isReplaced ? 1 : known.count + 1;

        return activity.call("occur", {
            id: known.id,
            ...ObjectReference.columns("actor", occurrence.actor),
            reason: notice.reason,
            payload: occurrence.payload,
            thread: occurrence.thread,
            count,
            firstOccurredAt: isReplaced ? occurrence.occurredAt : known.firstOccurredAt,
            occurredAt: occurrence.occurredAt,
            ...this.render(occurrence.payload, count, occurrence.locale),
        });
    }

    /** Read the announcement of a source under a key, absent before one. */
    async #announced(
        context: MethodContext,
        source: ObjectReference,
        key: string,
    ): Promise<Announcement | undefined> {
        const [known] = await context.read(announcement).findMany({
            where: {
                packageId: this.reference.packageId,
                name: this.name,
                parentPackageId: source.packageId,
                parentType: source.type,
                parentId: source.id,
                key,
            },
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
    /** When it occurred. */
    readonly occurredAt: number;
    /** Who caused it. */
    readonly actor: Subject;
}

/** The notify method's input: a notice's source, recipients, reason and payload. */
export type NotifyInput = MethodContextOf<typeof activity, "notify">["input"];

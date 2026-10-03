import { defineSchema, Instant, schema } from "@destack/schema";
import { DeclarationName, ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { LogPosition, type Condition } from "@destack/db";
import type { ResourceContext } from "@destack/resource/context";
import { Call } from "@destack/sync";
import {
    ChangeOn,
    type ChangeOperation,
    MAX_LAG_MILLISECONDS,
    type ObjectChange,
} from "./change.ts";
import type { WebhookDelivery, WebhookParameters } from "./webhook.ts";
import { WebhookOn } from "./webhook.ts";
import { ScheduleOn } from "./schedule.ts";

/** The digits a run event's numbers take in its request: every safe integer. */
const NUMBER_DIGITS = 16;

/** The kinds of events a trigger fires on. */
export const TRIGGER_KINDS = ["schedule", "change", "webhook"] as const;

/** A kind of event a trigger fires on. */
export type TriggerKind = (typeof TRIGGER_KINDS)[number];

/** What a trigger fires on: a schedule, an object's changes, or signed webhook deliveries. */
export const TriggerOn = defineSchema(
    schema.union([
        schema.object({
            /** The schedule whose occurrences fire. */
            schedule: ScheduleOn,
        }),
        schema.object({
            /** The changes that fire. */
            change: ChangeOn,
        }),
        schema.object({
            /** The signed deliveries that fire. */
            webhook: WebhookOn,
        }),
    ]),
);
/** What a trigger fires on. */
export type TriggerOn = schema.Infer<typeof TriggerOn>;

/** A trigger in the manifest. */
export const TriggerDescription = defineSchema(
    schema.object({
        /** The package-local trigger name. */
        name: DeclarationName,
        /** What the trigger fires on, a schedule with the call each occurrence runs. */
        on: TriggerOn,
    }),
);
/** A trigger in the manifest. */
export type TriggerDescription = schema.Infer<typeof TriggerDescription>;

/** A trigger firing on a schedule, each occurrence running the schedule's object method call. */
export interface ScheduleTrigger extends Declaration {
    /** The declaration kind. */
    readonly kind: "trigger";
    /** The schedule whose occurrences fire, with the call each runs. */
    readonly on: { readonly schedule: ScheduleOn };
}

/** A trigger firing on one object type's changes, each admitted change running one object method call. */
export interface ChangeTrigger<Target extends Declaration = Declaration> extends Declaration {
    /** The declaration kind. */
    readonly kind: "trigger";
    /** The changes that fire. */
    readonly on: {
        readonly change: {
            /** The changed object type. */
            readonly object: Target;
            /** The condition on the object's rows. */
            readonly where?: Condition;
            /** The operations fired on. */
            readonly operations: readonly ChangeOperation[];
            /** Where a new trigger starts: at the log's head, or with every matching row as created. */
            readonly from: "now" | "snapshot";
            /** How long the log keeps changes the trigger has not recorded, in milliseconds. */
            readonly maxLag: number;
        };
    };
    /** Build the call an admitted change runs. */
    call(change: ObjectChange<Target>): Call;
}

/** A trigger firing on signed webhook requests, each verified delivery running one object method call. */
export interface WebhookTrigger extends Declaration {
    /** The declaration kind. */
    readonly kind: "trigger";
    /** The signed deliveries that fire. */
    readonly on: {
        readonly webhook: WebhookOn & {
            /** Read the signing secret of a delivery's route parameters from the installation's resources. */
            readonly secret: (
                parameters: WebhookParameters,
                resources: ResourceContext,
            ) => Promise<string>;
        };
    };
    /** Build the call a verified delivery runs. */
    call(delivery: WebhookDelivery): Call;
}

/** A declared trigger: what fires it and the object method call each event runs. */
export type Trigger = ScheduleTrigger | ChangeTrigger | WebhookTrigger;

/** What a trigger fires on as its package writes it, before the defaults of a change trigger. */
export type TriggerOnDefinition<Target extends Declaration = Declaration> =
    | ScheduleTrigger["on"]
    | {
          readonly change: Omit<ChangeTrigger<Target>["on"]["change"], "from" | "maxLag"> &
              Partial<Pick<ChangeTrigger<Target>["on"]["change"], "from" | "maxLag">>;
      }
    | WebhookTrigger["on"];

/** A trigger as its package writes it, with the call matching what fires it. */
type WrittenTrigger = { readonly name: string } & (
    | { readonly on: ScheduleTrigger["on"] }
    | {
          readonly on: Extract<TriggerOnDefinition, { readonly change: unknown }>;
          readonly call: ChangeTrigger["call"];
      }
    | { readonly on: WebhookTrigger["on"]; readonly call: WebhookTrigger["call"] }
);

/** The triggers of each kind. */
export const Trigger = {
    /** Read the kind of event a trigger fires on. */
    kind(on: TriggerOn | Trigger["on"] | TriggerOnDefinition): TriggerKind {
        // read the key naming the kind
        if ("schedule" in on) {
            return "schedule";
        } else if ("change" in on) {
            return "change";
        } else {
            return "webhook";
        }
    },

    /** Report whether a trigger, as declared or as written, fires on a kind of event. */
    is<Value extends Trigger | WrittenTrigger, Kind extends TriggerKind>(
        trigger: Value,
        kind: Kind,
    ): trigger is Extract<Value, { readonly on: Record<Kind, object> }> {
        return Trigger.kind(trigger.on) === kind;
    },

    /** Select a trigger of a kind, absent for one of another kind. */
    of<Kind extends TriggerKind>(
        trigger: Trigger,
        kind: Kind,
    ): Extract<Trigger, { readonly on: Record<Kind, object> }> | undefined {
        return Trigger.is(trigger, kind) ? trigger : undefined;
    },
};

/** An event that fired a trigger: a schedule's occurrence, a verified delivery, or an admitted change. */
const event = defineSchema(
    schema.union([
        schema.object({
            /** The occurrence's time, in UTC epoch milliseconds. */
            at: Instant,
        }),
        schema.object({
            /** The verified delivery. */
            delivery: schema.object({
                /** The sender's delivery identifier. */
                id: schema.string().min(1),
                /** The route path it arrived at. */
                path: schema.string().min(1),
            }),
        }),
        schema.object({
            /** The admitted change. */
            change: schema.object({
                /** The change's position in the log. */
                position: LogPosition,
                /** The row a snapshot delivers at the position, absent for a logged change. */
                key: schema.string().min(1).exactOptional(),
            }),
        }),
    ]),
);
/** An event that fired a trigger. */
export type RunEvent = schema.Infer<typeof event>;

/** An event that fired a trigger, and the request recording its run once. */
export const RunEvent = Object.assign(event, {
    /** Derive the request an event records its run under: one per trigger, in log order for changes. */
    requestId(fired: RunEvent): string {
        // key an occurrence by its time
        if ("at" in fired) {
            return String(fired.at).padStart(NUMBER_DIGITS, "0");
        }
        // key a delivery by its path and the sender's identifier
        else if ("delivery" in fired) {
            return `${fired.delivery.path} ${fired.delivery.id}`;
        }
        // key a change by its position and snapshot row
        else {
            const { position, key } = fired.change;
            const sequence = String(position.sequence).padStart(NUMBER_DIGITS, "0");

            return `${position.epoch}/${sequence}/${key ?? ""}`;
        }
    },
});

/** A call sent to run later, on the lent authority of the caller whose call sent it. */
const SENT = schema.object({
    /** The object method call the run makes. */
    call: Call,
    /** When the call runs in UTC epoch milliseconds, at recording when absent. */
    at: Instant.exactOptional(),
    /** The lending of the sending caller's authority, so the call runs on that caller's behalf. */
    delegation: schema.string().min(1).exactOptional(),
});

/** A call one of the installation's triggers runs for an event, once per event. */
const FIRED = schema.object({
    /** The object method call the run makes. */
    call: Call,
    /** The name the installation's package declares the trigger under. */
    triggerName: DeclarationName,
    /** The event that fired it. */
    event: RunEvent,
});

/** A call to run later, sent or fired by a trigger. */
export const RunRequest = Object.assign(defineSchema(schema.union([SENT, FIRED])), {
    /** A call sent to run later. */
    sent: SENT,
    /** A call a trigger fired for an event. */
    fired: FIRED,
});
/** A call to run later. */
export type RunRequest = schema.Infer<typeof RunRequest>;

/** How a request reaches the cell. */
export interface RunDelivery {
    /** The request recording the run once however often it is delivered, a new one when absent. */
    readonly requestId?: string;
    /** Abort the delivery. */
    readonly signal?: AbortSignal;
}

/** The cell recording an installation's runs, as its workload reaches it. */
export interface RunClient {
    /** Record a call to run later, once per request and per event of a trigger. */
    send(request: RunRequest, delivery?: RunDelivery): Promise<void>;
}

/** Declare a trigger firing on a schedule, its call inside the schedule. */
export function defineTrigger(
    definition: { readonly name: string; readonly on: ScheduleTrigger["on"] },
    module?: ModuleMetadata,
): Trigger;
/** Declare a trigger firing on an object's changes, with the code building each change's call. */
export function defineTrigger<const Target extends Declaration>(
    definition: {
        readonly name: string;
        readonly on: Extract<TriggerOnDefinition<Target>, { readonly change: unknown }>;
        readonly call: (change: ObjectChange<Target>) => Call;
    },
    module?: ModuleMetadata,
): Trigger;
/** Declare a trigger firing on webhook deliveries, with the code building each delivery's call. */
export function defineTrigger(
    definition: {
        readonly name: string;
        readonly on: WebhookTrigger["on"];
        readonly call: (delivery: WebhookDelivery) => Call;
    },
    module?: ModuleMetadata,
): Trigger;
/** Declare a trigger: a schedule with its call, or changes or webhook deliveries with the code building theirs. */
export function defineTrigger(definition: WrittenTrigger, module?: ModuleMetadata): Trigger {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineTrigger").package;
    const name = DeclarationName.parse(definition.name);
    const declared = { kind: "trigger" as const, name, package: owner };

    // check a schedule against its schema
    if (Trigger.is(definition, "schedule")) {
        const trigger: ScheduleTrigger = {
            ...declared,
            on: { schedule: ScheduleOn.parse(definition.on.schedule) },
        };

        return Object.freeze(trigger);
    }
    // check a webhook's route
    else if (Trigger.is(definition, "webhook")) {
        const { secret, ...fields } = definition.on.webhook;
        WebhookOn.parameters(WebhookOn.parse(fields).route);
        const trigger: WebhookTrigger = {
            ...declared,
            on: { webhook: { ...fields, secret } },
            call: definition.call,
        };

        return Object.freeze(trigger);
    }
    // require a change trigger
    else if (!Trigger.is(definition, "change")) {
        throw new TypeError(`trigger ${name} fires on no known event`);
    }

    // default where a change trigger starts and how far it may lag, requiring creations for a snapshot
    const { object, where, ...fields } = definition.on.change;
    const change = ChangeOn.omit({ object: true, where: true }).parse({
        from: "now",
        maxLag: MAX_LAG_MILLISECONDS,
        ...fields,
    });
    if (change.from === "snapshot" && !change.operations.includes("create")) {
        throw new TypeError(
            `trigger ${name} starts with a snapshot, whose rows are created, but fires on no creations`,
        );
    }

    // keep the change trigger's call
    const trigger: ChangeTrigger = {
        ...declared,
        on: { change: { ...change, object, ...(where === undefined ? {} : { where }) } },
        call: definition.call,
    };

    return Object.freeze(trigger);
}

import { defineSchema, Instant, schema } from "@destack/schema";
import { DeclarationName, ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { LogPosition } from "@destack/db/log";
import type { Condition } from "@destack/db/query";
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
        /** What the trigger fires on. */
        on: TriggerOn,
        /** The call each occurrence of a schedule runs, absent for triggers whose code builds it. */
        call: Call.optional(),
    }),
);
/** A trigger in the manifest. */
export type TriggerDescription = schema.Infer<typeof TriggerDescription>;

/** A trigger firing on a schedule, each occurrence running one object method call. */
export interface ScheduleTrigger extends Declaration {
    /** The declaration kind. */
    readonly kind: "trigger";
    /** The schedule whose occurrences fire. */
    readonly on: { readonly schedule: ScheduleOn };
    /** The call each occurrence runs, in the installation's space. */
    readonly call: Call;
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
            secret(parameters: WebhookParameters, resources: ResourceContext): Promise<string>;
        };
    };
    /** Build the call a verified delivery runs. */
    call(delivery: WebhookDelivery): Call;
}

/** A declared trigger: what fires it and the object method call each event runs. */
export type Trigger = ScheduleTrigger | ChangeTrigger<any> | WebhookTrigger;

/** What a trigger fires on as its package writes it, before the defaults of a change trigger. */
export type TriggerOnDefinition<Target extends Declaration = Declaration> =
    | ScheduleTrigger["on"]
    | {
          readonly change: Omit<ChangeTrigger<Target>["on"]["change"], "from" | "maxLag"> &
              Partial<Pick<ChangeTrigger<Target>["on"]["change"], "from" | "maxLag">>;
      }
    | WebhookTrigger["on"];

/** The call a trigger runs for what fires it: a schedule's call, or the code building a change's or a delivery's. */
export type TriggerCall<On> = On extends { readonly schedule: unknown }
    ? Call
    : On extends { readonly change: { readonly object: infer Target } }
      ? (change: ObjectChange<Target>) => Call
      : (delivery: WebhookDelivery) => Call;

/** A trigger as its package writes it. */
export interface TriggerDefinition<On> {
    /** The package-local trigger name. */
    readonly name: string;
    /** What fires the trigger. */
    readonly on: On;
    /** The call each event runs. */
    readonly call: TriggerCall<On>;
}

/** The triggers of each kind. */
export const Trigger = {
    /** Read the kind of event a trigger fires on. */
    kind(on: TriggerOn | Trigger["on"]): TriggerKind {
        return "schedule" in on ? "schedule" : "change" in on ? "change" : "webhook";
    },

    /** Select a trigger of a kind, absent for one of another kind. */
    of<Kind extends TriggerKind>(
        trigger: Trigger,
        kind: Kind,
    ): Extract<Trigger, { readonly on: Record<Kind, unknown> }> | undefined {
        return Trigger.kind(trigger.on) === kind ? (trigger as never) : undefined;
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
                key: schema.string().min(1).optional(),
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
    /** When the call runs, in UTC epoch milliseconds; when the cell records it when absent. */
    at: Instant.optional(),
    /** The lending of the sending caller's authority, so the call runs on that caller's behalf. */
    delegation: schema.string().min(1).optional(),
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
export const RunRequest = defineSchema(schema.union([SENT, FIRED]));
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

/** Declare a trigger: a schedule with its call, or changes or webhook deliveries with the code building theirs. */
export function defineTrigger<const On extends TriggerOnDefinition<any>>(
    definition: TriggerDefinition<On>,
    module?: ModuleMetadata,
): Trigger {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineTrigger").package;
    const name = DeclarationName.parse(definition.name);
    const declared = { kind: "trigger" as const, name, package: owner, call: definition.call };
    const on = definition.on as TriggerOnDefinition;

    // check a schedule against its schema
    if ("schedule" in on) {
        return Object.freeze({
            ...declared,
            on: { schedule: ScheduleOn.parse(on.schedule) },
        }) as ScheduleTrigger;
    }
    // check a webhook's route
    else if ("webhook" in on) {
        const { secret, ...fields } = on.webhook;
        WebhookOn.parameters(WebhookOn.parse(fields).route);

        return Object.freeze({
            ...declared,
            on: { webhook: { ...fields, secret } },
        }) as WebhookTrigger;
    }

    // default where a change trigger starts and how far it may lag, requiring creations for a snapshot
    const { object, where, ...fields } = on.change;
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

    return Object.freeze({
        ...declared,
        on: { change: { ...change, object, ...(where === undefined ? {} : { where }) } },
    }) as ChangeTrigger;
}

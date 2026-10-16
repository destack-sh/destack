import type { Condition } from "@destack/db";
import { DeclarationName } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import type { ServerContext } from "@destack/runtime/workload";
import { defineSchema, schema } from "@destack/schema";
import {
    ChangeOn,
    type ChangeOperation,
    MAX_LAG_MILLISECONDS,
    type ObjectChange,
} from "./change.ts";
import { ScheduleOn } from "./schedule.ts";
import {
    type WebhookDelivery,
    WebhookOn,
    type WebhookParameters,
    type WebhookSignature,
} from "./webhook.ts";

/** The digits an event's numbers take in its key: every safe integer. */
const NUMBER_DIGITS = 16;

/** The kinds of events a trigger fires on. */
export const TRIGGER_KINDS = ["schedule", "change", "webhook"] as const;

/** A kind of event a trigger fires on. */
export type TriggerKind = (typeof TRIGGER_KINDS)[number];

/** A workflow's trigger in the manifest: its name and what fires it, by the kind of event. */
export const TriggerDescription = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The trigger's name within its workflow. */
            name: DeclarationName,
            /** A trigger firing on a schedule. */
            kind: schema.literal("schedule"),
            /** The schedule whose occurrences fire. */
            schedule: ScheduleOn,
        }),
        schema.object({
            /** The trigger's name within its workflow. */
            name: DeclarationName,
            /** A trigger firing on an object type's changes. */
            kind: schema.literal("change"),
            /** The changes that fire. */
            change: ChangeOn,
        }),
        schema.object({
            /** The trigger's name within its workflow. */
            name: DeclarationName,
            /** A trigger firing on signed webhook deliveries. */
            kind: schema.literal("webhook"),
            /** The signed deliveries that fire. */
            webhook: WebhookOn,
        }),
    ]),
);
/** A workflow's trigger in the manifest. */
export type TriggerDescription = schema.Infer<typeof TriggerDescription>;

/** The changes of one object type a trigger fires on, as its package declares them. */
export interface ChangeTriggerOn {
    /** The changed object type. */
    readonly object: Declaration;
    /** The condition on the object's rows. */
    readonly where?: Condition;
    /** The operations fired on. */
    readonly operations: readonly ChangeOperation[];
    /** Where a new trigger starts: at the log's head, now by default, or with a snapshot. */
    readonly from?: "now" | "snapshot";
    /** How long the log retains changes the trigger has not taken, in milliseconds. */
    readonly maxLag?: number;
}

/** The signed deliveries a trigger fires on, as its package declares them. */
export interface WebhookTriggerOn extends Pick<WebhookOn, "path"> {
    /** The scheme verifying each delivery's signature. */
    readonly signature: WebhookSignature;
    /** Read the signing secret of a delivery's route parameters. */
    readonly secret?: (
        parameters: WebhookParameters,
        context: Pick<ServerContext, "resources" | "keys">,
    ) => Promise<string>;
}

/** A trigger firing on a schedule's occurrences. */
export interface ScheduleTrigger<Input = unknown> {
    /** The trigger's name within its workflow. */
    readonly name: string;
    /** A trigger firing on a schedule. */
    readonly kind: "schedule";
    /** The schedule whose occurrences fire. */
    readonly schedule: ScheduleOn;
    /** Read the input of an occurrence's run. */
    readonly input: (occurrence: { readonly at: number }) => Input;
}

/** A trigger firing on one object type's changes. */
export interface ChangeTrigger<Input = unknown> {
    /** The trigger's name within its workflow. */
    readonly name: string;
    /** A trigger firing on an object type's changes. */
    readonly kind: "change";
    /** The changes that fire, the defaults filled in. */
    readonly change: Required<Omit<ChangeTriggerOn, "where">> & Pick<ChangeTriggerOn, "where">;
    /** Read the input of a change's run. */
    readonly input: (change: ObjectChange) => Input;
}

/** A trigger firing on signed webhook deliveries. */
export interface WebhookTrigger<Input = unknown> {
    /** The trigger's name within its workflow. */
    readonly name: string;
    /** A trigger firing on signed webhook deliveries. */
    readonly kind: "webhook";
    /** The signed deliveries that fire. */
    readonly webhook: WebhookTriggerOn;
    /** Read the input of a delivery's run. */
    readonly input: (delivery: WebhookDelivery) => Input;
}

/** A workflow's declared trigger: what fires it, and the input of the run each event starts. */
export type Trigger<Input = unknown> =
    | ScheduleTrigger<Input>
    | ChangeTrigger<Input>
    | WebhookTrigger<Input>;

/** An event that fired a trigger: an occurrence, a change or a delivery. */
export type TriggerEvent = { readonly at: number } | ObjectChange | WebhookDelivery;

/** A workflow's trigger as the workflow declares it, a change trigger's defaults left out. */
export type TriggerDefinition<Input> =
    | Omit<ScheduleTrigger<Input>, "name">
    | (Omit<ChangeTrigger<Input>, "name" | "change"> & { readonly change: ChangeTriggerOn })
    | Omit<WebhookTrigger<Input>, "name">;

/** The triggers of each kind and the events firing them. */
export const Trigger = {
    /** Check a workflow's trigger as declared, filling a change trigger's defaults. */
    of<Input>(name: string, definition: TriggerDefinition<Input>): Trigger<Input> {
        // check a schedule against its schema
        const checked = DeclarationName.parse(name);
        if (definition.kind === "schedule") {
            const schedule = ScheduleOn.parse(definition.schedule);

            return { name: checked, kind: "schedule", schedule, input: definition.input };
        }
        // check a webhook's path template
        else if (definition.kind === "webhook") {
            const { signature, path, secret } = definition.webhook;
            WebhookOn.parameters(WebhookOn.parse({ verification: signature.name, path }).path);
            const webhook = { signature, path, ...(secret === undefined ? {} : { secret }) };

            return { name: checked, kind: "webhook", webhook, input: definition.input };
        }

        // default where a change trigger starts and how far it may lag, refusing a snapshot without creations
        const { object, where, from = "now", maxLag = MAX_LAG_MILLISECONDS } = definition.change;
        const { operations } = definition.change;
        if (from === "snapshot" && !operations.includes("create")) {
            throw new TypeError(
                `trigger ${checked} starts with a snapshot of created rows but does not fire on creations`,
            );
        }
        const change = {
            object,
            operations: [...operations],
            from,
            maxLag,
            ...(where === undefined ? {} : { where }),
        };

        return { name: checked, kind: "change", change, input: definition.input };
    },

    /** Key an event: an occurrence by its time, a change by its position, a delivery by its id. */
    key(event: TriggerEvent): string {
        // key an occurrence by its time
        if ("at" in event) {
            return String(event.at).padStart(NUMBER_DIGITS, "0");
        }
        // key a delivery by its path's parameters and the sender's identifier
        else if ("receivedAt" in event) {
            return `${JSON.stringify(event.parameters)} ${event.id}`;
        }
        // key a change by its position and snapshot row
        else {
            const sequence = String(event.position.sequence).padStart(NUMBER_DIGITS, "0");
            const row = event.key === undefined ? "" : schema.string().parse(event.key["id"]);

            return `${event.position.epoch}/${sequence}/${row}`;
        }
    },
};

import type { Condition } from "@destack/db";
import { DeclarationName } from "@destack/package";
import { schema } from "@destack/schema";
import { ChangeOn, type ChangeOperation, type ObjectChange } from "./change.ts";
import { Schedule, ScheduleOn } from "./schedule.ts";
import { type Declaration, DeclarationReference } from "@destack/package/declare";
import type { Webhook, WebhookDelivery } from "../webhook/webhook.ts";

/** The digits an event's numbers take in its key: every safe integer. */
const NUMBER_DIGITS = 16;

/** The kinds of events a trigger fires on. */
export const TRIGGER_KINDS = ["schedule", "change", "webhook"] as const;

/** A kind of event a trigger fires on. */
export type TriggerKind = (typeof TRIGGER_KINDS)[number];

/** A workflow's trigger in the manifest: its name and what fires it, by the kind of event. */
export const TriggerDescription = schema.discriminatedUnion("kind", [
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
        /** A trigger firing on a webhook's verified deliveries. */
        kind: schema.literal("webhook"),
        /** The webhook whose deliveries fire. */
        webhook: DeclarationReference,
    }),
]);
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
}

/** A trigger firing on a schedule's occurrences. */
export class ScheduleTrigger<Input = unknown> {
    /** A trigger firing on a schedule. */
    readonly kind: "schedule";
    /** The trigger's name within its workflow. */
    readonly name: string;
    /** The schedule whose occurrences fire. */
    readonly schedule: ScheduleOn;
    /** Read the input of an occurrence's run. */
    readonly input: (occurrence: { readonly at: number }) => Input;

    /** Check a schedule trigger as its workflow declares it. */
    constructor(name: string, definition: Omit<ScheduleTrigger<Input>, "name" | "kind">) {
        // check the name and the schedule
        this.kind = "schedule";
        this.name = DeclarationName.parse(name);
        this.schedule = ScheduleOn.parse(definition.schedule);
        Schedule.require(this.schedule.timing);
        this.input = definition.input;
    }

    /** Key an occurrence by its time. */
    static key(occurrence: { readonly at: number }): string {
        return String(occurrence.at).padStart(NUMBER_DIGITS, "0");
    }
}

/** A trigger firing on one object type's changes. */
export class ChangeTrigger<Input = unknown> {
    /** A trigger firing on an object type's changes. */
    readonly kind: "change";
    /** The trigger's name within its workflow. */
    readonly name: string;
    /** The changes that fire, the defaults filled in. */
    readonly change: Required<Omit<ChangeTriggerOn, "where">> & Pick<ChangeTriggerOn, "where">;
    /** Read the input of a change's run. */
    readonly input: (change: ObjectChange) => Input;

    /** Check a change trigger as declared, refusing a snapshot without creations. */
    constructor(
        name: string,
        definition: {
            readonly change: ChangeTriggerOn;
            readonly input: (change: ObjectChange) => Input;
        },
    ) {
        // default where the trigger starts, refusing a snapshot of rows it does not fire on
        const checked = DeclarationName.parse(name);
        const { object, where, from = "now", operations } = definition.change;
        if (from === "snapshot" && !operations.includes("create")) {
            throw new TypeError(
                `trigger '${checked}' starts with a snapshot of created rows but does not fire on creations`,
            );
        }
        this.kind = "change";
        this.name = checked;
        this.change = {
            object,
            operations: [...operations],
            from,
            ...(where === undefined ? {} : { where }),
        };
        this.input = definition.input;
    }

    /** Key a change by its position and snapshot row. */
    static key(change: ObjectChange): string {
        const sequence = String(change.position.sequence).padStart(NUMBER_DIGITS, "0");
        const row = change.key === undefined ? "" : schema.string().parse(change.key["id"]);

        return `${change.position.epoch}/${sequence}/${row}`;
    }
}

/** A trigger firing on a webhook's verified deliveries. */
export class WebhookTrigger<Input = unknown> {
    /** A trigger firing on a webhook's verified deliveries. */
    readonly kind: "webhook";
    /** The trigger's name within its workflow. */
    readonly name: string;
    /** The webhook whose deliveries fire. */
    readonly webhook: Webhook;
    /** Read the input of a delivery's run. */
    readonly input: (delivery: WebhookDelivery) => Input;

    /** Check a webhook trigger as its workflow declares it. */
    constructor(name: string, definition: Omit<WebhookTrigger<Input>, "name" | "kind">) {
        // check the name, and take the webhook and the input
        this.kind = "webhook";
        this.name = DeclarationName.parse(name);
        this.webhook = definition.webhook;
        this.input = definition.input;
    }

    /** Key a delivery by its path's parameters and the sender's identifier. */
    static key(delivery: WebhookDelivery): string {
        return `${JSON.stringify(delivery.parameters)} ${delivery.id}`;
    }
}

/** A workflow's declared trigger: what fires it, and the input of the run each event starts. */
export type Trigger<Input = unknown> =
    | ScheduleTrigger<Input>
    | ChangeTrigger<Input>
    | WebhookTrigger<Input>;

/** A workflow's trigger as the workflow declares it, a change trigger's defaults left out. */
export type TriggerDefinition<Input> =
    | ({ readonly kind: "schedule"; readonly name: string } & Omit<
          ScheduleTrigger<Input>,
          "name" | "kind"
      >)
    | {
          readonly kind: "change";
          readonly name: string;
          readonly change: ChangeTriggerOn;
          readonly input: (change: ObjectChange) => Input;
      }
    | ({ readonly kind: "webhook"; readonly name: string } & Omit<
          WebhookTrigger<Input>,
          "name" | "kind"
      >);

/** Check a workflow's declared trigger as the noun of its kind. */
export function triggerOf<Input>(definition: TriggerDefinition<Input>): Trigger<Input> {
    const { name } = definition;

    // check a schedule
    if (definition.kind === "schedule") {
        return new ScheduleTrigger(name, definition);
    }
    // check a webhook
    else if (definition.kind === "webhook") {
        return new WebhookTrigger(name, definition);
    }
    // check a change
    else {
        return new ChangeTrigger(name, definition);
    }
}

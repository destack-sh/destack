import { handleTrigger, type Handled } from "../trigger/trigger.ts";
import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, declaringModule, type ModuleMetadata } from "@destack/package";
import { type Declaration, DeclarationReference } from "@destack/package/declare";

/** The signature schemes a webhook's sender may prove deliveries with. */
export const WEBHOOK_VERIFICATIONS = ["standard", "github"] as const;

/** A webhook, as the manifest describes it. */
export const WebhookDescription = defineSchema(
    schema.object({
        /** The package-local webhook name. */
        name: DeclarationName,
        /** The declaration format version. */
        version: schema.literal(1),
        /** The scheme proving each delivery: Standard Webhooks, or GitHub's signature. */
        verification: schema.enum(WEBHOOK_VERIFICATIONS),
        /** The secret the sender signs deliveries with. */
        secret: DeclarationReference,
    }),
);
/** A webhook, as the manifest describes it. */
export type WebhookDescription = schema.Infer<typeof WebhookDescription>;

/** A declared event source whose sender posts signed deliveries, received by the workload implementing it. */
export interface Webhook extends Declaration, Handled<Webhook> {
    /** The trigger kind. */
    readonly kind: "webhook";
    /** The declaration format version. */
    readonly version: 1;
    /** The scheme proving each delivery. */
    readonly verification: WebhookDescription["verification"];
    /** The secret declaration holding the signing secret, bound per installation. */
    readonly secret: Declaration;
}

/** One verified webhook delivery, which the host delivers once per identifier. */
export const WebhookDelivery = defineSchema(
    schema.object({
        /** The sender's delivery identifier, the same on every retry of one event. */
        id: schema.string().min(1),
        /** The event type, such as push. */
        event: schema.string().min(1),
        /** The decoded body. */
        payload: schema.json(),
        /** The time the host received the delivery in UTC epoch milliseconds. */
        receivedAt: schema.number().int(),
    }),
);
/** One verified webhook delivery, which the host delivers once per identifier. */
export type WebhookDelivery = schema.Infer<typeof WebhookDelivery>;

/** Declare a webhook whose deliveries the implementing workload receives. */
export function defineWebhook(
    definition: Pick<Webhook, "name" | "verification" | "secret">,
    module?: ModuleMetadata,
): Webhook {
    // stamp the declaring package supplied by the module transform
    const owner = declaringModule(module, "defineWebhook").package;
    const { secret, ...fields } = definition;
    const description = WebhookDescription.omit({ secret: true }).parse({ ...fields, version: 1 });

    return Object.freeze({
        ...description,
        kind: "webhook",
        secret,
        package: owner,
        handle: handleTrigger,
    });
}

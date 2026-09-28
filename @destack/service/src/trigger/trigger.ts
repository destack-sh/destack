import type { Schedule, ScheduleOccurrence } from "../schedule/schedule.ts";
import type { Webhook, WebhookHandler } from "../webhook/webhook.ts";
import type { WebhookDelivery } from "../webhook/delivery.ts";
import type { Watch, ObjectChange } from "../watch/watch.ts";

/** The trigger kinds. */
export const TRIGGER_KINDS = ["schedule", "webhook", "watch"] as const;

/** A declared event source. */
export type Trigger = Schedule | Webhook | Watch<any>;

/** The declared triggers of one kind. */
export type TriggerOf<Kind extends Trigger["kind"]> = Extract<Trigger, { readonly kind: Kind }>;

/** The event of one trigger delivery. */
export type TriggerEvent<Declared> =
    Declared extends Watch<infer Target>
        ? ObjectChange<Target>
        : Declared extends { readonly kind: "schedule" }
          ? ScheduleOccurrence
          : Declared extends { readonly kind: "webhook" }
            ? WebhookDelivery
            : never;

/** A trigger that pairs with its handler. */
export interface Handled<Declared> {
    /** Pair the trigger with its handler. */
    handle(
        handle: (event: TriggerEvent<Declared>, signal: AbortSignal) => Promise<void>,
    ): TriggerHandler<Declared>;
}

/** Pair a trigger with its handler. */
export function handleTrigger<Declared extends Trigger>(
    this: Declared,
    handle: (event: TriggerEvent<Declared>, signal: AbortSignal) => Promise<void>,
): TriggerHandler<Declared> {
    return { trigger: this, handle };
}

/** A workload's handler of one trigger. */
export interface TriggerHandler<Declared> {
    /** The trigger. */
    readonly trigger: Declared;
    /** Handle one event. */
    handle(event: TriggerEvent<Declared>, signal: AbortSignal): Promise<void>;
}

/** A workload's handler of one trigger, of any kind. */
export type TriggerImplementation = {
    [Kind in Trigger["kind"]]: Kind extends "webhook"
        ? WebhookHandler
        : TriggerHandler<TriggerOf<Kind>>;
}[Trigger["kind"]];

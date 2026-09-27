import type { Schedule, ScheduleOccurrence } from "../schedule/schedule.ts";
import type { Webhook, WebhookDelivery } from "../webhook/webhook.ts";
import type { Subscription, SubscriptionChange } from "../subscription/subscription.ts";

/** The kinds of event sources a host delivers to workloads. */
export const TRIGGER_KINDS = ["schedule", "webhook", "subscription"] as const;

/** A declared event source the host delivers to the workload implementing it. */
export type Trigger = Schedule | Webhook | Subscription<any>;

/** The declared triggers of one kind. */
export type TriggerOf<Kind extends Trigger["kind"]> = Extract<Trigger, { readonly kind: Kind }>;

/** The event one delivery of a trigger carries. */
export type TriggerEvent<Declared> =
    Declared extends Subscription<infer Target>
        ? SubscriptionChange<Target>
        : Declared extends { readonly kind: "schedule" }
          ? ScheduleOccurrence
          : Declared extends { readonly kind: "webhook" }
            ? WebhookDelivery
            : never;

/** A declared trigger that pairs itself with its workload's handler. */
export interface Handled<Declared> {
    /** Pair the trigger with the handler of its events, typed by the trigger's kind. */
    handle(
        handle: (event: TriggerEvent<Declared>, signal: AbortSignal) => Promise<void>,
    ): TriggerHandler<Declared>;
}

/** Pair a trigger with the handler of its events. */
export function handleTrigger<Declared extends Trigger>(
    this: Declared,
    handle: (event: TriggerEvent<Declared>, signal: AbortSignal) => Promise<void>,
): TriggerHandler<Declared> {
    return { trigger: this, handle };
}

/** A workload's handler for one declared trigger of a kind. */
export interface TriggerHandler<Declared> {
    /** The declared trigger this handler handles. */
    readonly trigger: Declared;
    /** Handle one delivered event, observing cancellation. */
    handle(event: TriggerEvent<Declared>, signal: AbortSignal): Promise<void>;
}

/** A workload's handler for one declared trigger, of any kind. */
export type TriggerImplementation = {
    [Kind in Trigger["kind"]]: TriggerHandler<TriggerOf<Kind>>;
}[Trigger["kind"]];

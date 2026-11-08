/** The Checkout events finance follows. */
export const CHECKOUT_EVENTS = ["checkout.session.completed", "checkout.session.expired"] as const;

/** The subscription events finance follows. */
export const SUBSCRIPTION_EVENTS: ReadonlySet<string> = new Set([
    "customer.subscription.updated",
    "customer.subscription.deleted",
    "customer.subscription.paused",
    "customer.subscription.resumed",
]);

/** The invoice events finance mirrors. */
export const INVOICE_EVENTS: ReadonlySet<string> = new Set([
    "invoice.created",
    "invoice.finalized",
    "invoice.paid",
    "invoice.payment_failed",
    "invoice.voided",
    "invoice.marked_uncollectible",
]);

/** The event types finance follows, which each home's webhook endpoint subscribes to. */
export const STRIPE_EVENTS: readonly string[] = [
    ...CHECKOUT_EVENTS,
    ...SUBSCRIPTION_EVENTS,
    ...INVOICE_EVENTS,
    "account.updated",
];

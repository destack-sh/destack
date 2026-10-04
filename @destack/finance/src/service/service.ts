import { defineService } from "@destack/service";
import {
    customer,
    entitlement,
    invoice,
    meterEvent,
    price,
    product,
    productFeature,
    purchase,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** Customers, sellers, their products and prices, subscriptions, purchases, entitlements, meter events and invoices, served as objects. */
export const financeService = defineService("finance", {
    objects: {
        customer,
        seller,
        product,
        productFeature,
        price,
        subscription,
        subscriptionItem,
        purchase,
        entitlement,
        meterEvent,
        invoice,
    },
});

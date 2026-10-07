import { allowance } from "@destack/account/object";
import { defineService } from "@destack/service";
import {
    charge,
    customer,
    entitlement,
    invoice,
    price,
    product,
    productFeature,
    checkoutSession,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** The finance objects. */
export const financeService = defineService("finance", {
    objects: {
        customer,
        seller,
        product,
        productFeature,
        price,
        subscription,
        subscriptionItem,
        checkoutSession,
        entitlement,
        allowance,
        charge,
        invoice,
    },
});

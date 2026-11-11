import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The finance package's failures, each reason with the call error code it answers. */
export const FinanceError = defineErrorFamily({
    reasons: {
        // not found
        CHECKOUT_NOT_FOUND: {},
        CUSTOMER_NOT_FOUND: {},
        FEATURE_NOT_FOUND: {},
        METER_NOT_FOUND: {},
        PRICE_NOT_FOUND: {},
        PRODUCT_NOT_FOUND: {},
        SELLER_NOT_FOUND: {},
        SKU_NOT_FOUND: {},
        STRIPE_NOT_FOUND: {},

        // exists
        SUBSCRIPTION_EXISTS: {},

        // conflict
        USAGE_CONFLICT: {},

        // invalid
        AGGREGATION_INVALID: {},
        COUNTRY_INVALID: {},
        CURRENCY_INVALID: {},
        LEVEL_INVALID: {},
        METER_INVALID: {},
        PRICE_INVALID: {},
        PRODUCT_INVALID: {},
        USAGE_INVALID: {},
        VALUE_INVALID: {},

        // required
        PRICE_REQUIRED: {},

        // denied
        PUBLISHER_DENIED: {},
        SELLER_DENIED: {},

        // blocked
        CHECKOUT_BLOCKED: {},
        CONVERSION_BLOCKED: {},
        CUSTOMER_BLOCKED: {},
        PROVIDER_BLOCKED: {},
        SELLER_BLOCKED: {},
        UNITS_BLOCKED: {},

        // unavailable
        PERIOD_UNAVAILABLE: {},
        SELLER_UNAVAILABLE: {},
        USAGE_UNAVAILABLE: {},

        // unreachable
        STRIPE_UNREACHABLE: {},
    },
});

/** A stable cause of finance failures. */
export type FinanceReason = ReasonOf<typeof FinanceError>;

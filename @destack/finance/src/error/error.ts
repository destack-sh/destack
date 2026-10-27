import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The finance package's failures, each reason with the call error code it answers. */
export const FinanceError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    AGGREGATION_MISMATCH: { code: "BAD_REQUEST" },
    CURRENCY_UNPRICED: { code: "BAD_REQUEST" },
    DEFAULT_PRODUCT_PRICED: { code: "BAD_REQUEST" },
    INCLUDED_USAGE_INVALID: { code: "BAD_REQUEST" },
    LEVEL_MISMATCH: { code: "BAD_REQUEST" },
    LICENSED_PRICE_MISSING: { code: "BAD_REQUEST" },
    METER_MISMATCH: { code: "BAD_REQUEST" },
    PRICE_INACTIVE: { code: "BAD_REQUEST" },
    PRODUCT_INACTIVE: { code: "BAD_REQUEST" },
    USAGE_BILLED: { code: "BAD_REQUEST" },
    VALUE_REFUSED: { code: "BAD_REQUEST" },

    // forbidden
    PUBLISHER_MISMATCH: { code: "FORBIDDEN" },

    // not found
    CHECKOUT_NOT_FOUND: { code: "NOT_FOUND" },
    CUSTOMER_NOT_FOUND: { code: "NOT_FOUND" },
    FEATURE_UNDECLARED: { code: "NOT_FOUND" },
    METER_UNDECLARED: { code: "NOT_FOUND" },
    PRICE_NOT_FOUND: { code: "NOT_FOUND" },
    PRICE_NOT_MIRRORED: { code: "NOT_FOUND" },
    PRODUCT_NOT_FOUND: { code: "NOT_FOUND" },
    PRODUCT_UNAVAILABLE: { code: "NOT_FOUND" },
    SELLER_NOT_FOUND: { code: "NOT_FOUND" },
    SKU_UNDECLARED: { code: "NOT_FOUND" },
    STRIPE_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    CHECKOUT_COMPLETE: { code: "CONFLICT" },
    SUBSCRIPTION_DUPLICATE: { code: "CONFLICT" },
    USAGE_CHANGED: { code: "CONFLICT" },

    // precondition failed
    CONVERSION_UNPRICED: { code: "PRECONDITION_FAILED" },
    COUNTRY_IMMUTABLE: { code: "PRECONDITION_FAILED" },
    CUSTOMER_MISSING: { code: "PRECONDITION_FAILED" },
    PROVIDER_MISSING: { code: "PRECONDITION_FAILED" },
    SELLER_INACTIVE: { code: "PRECONDITION_FAILED" },
    SELLER_MISSING: { code: "PRECONDITION_FAILED" },
    SELLER_PLATFORM: { code: "PRECONDITION_FAILED" },
    UNITS_MIXED: { code: "PRECONDITION_FAILED" },

    // bad gateway
    STRIPE_FAILED: { code: "BAD_GATEWAY" },

    // service unavailable
    PERIOD_CLOSED: { code: "SERVICE_UNAVAILABLE" },
    PERIOD_ENDED: { code: "SERVICE_UNAVAILABLE" },
    SELLER_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
    USAGE_AHEAD: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of finance failures. */
export type FinanceReason = ReasonOf<typeof FinanceError>;

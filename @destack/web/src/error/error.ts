import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The web package's failures, each reason with the call error code it answers. */
export const WebError = defineErrorFamily(
    import.meta.destack.package,
    {
        // bad request
        DUPLICATE_PAGE: { code: "BAD_REQUEST" },
        FEED_EMPTY: { code: "BAD_REQUEST" },
        INVALID_FEED_DATE: { code: "BAD_REQUEST" },
        INVALID_OPTIONS: { code: "BAD_REQUEST" },
        INVALID_SECURITY_EXPIRY: { code: "BAD_REQUEST" },
        ROUTE_NOT_CANONICAL: { code: "BAD_REQUEST" },
        SITE_REQUIRED: { code: "BAD_REQUEST" },
        SSR_REQUIRED: { code: "BAD_REQUEST" },

        // unprocessable content
        DIAGNOSTICS_TOO_LONG: { code: "UNPROCESSABLE_CONTENT" },
        PERSONALIZED_RESPONSE: { code: "UNPROCESSABLE_CONTENT" },
        PRERENDER_EXITED: { code: "UNPROCESSABLE_CONTENT" },
        PRERENDER_UNREADABLE: { code: "UNPROCESSABLE_CONTENT" },
        UNEXPECTED_RESPONSE: { code: "UNPROCESSABLE_CONTENT" },

        // gateway timeout
        PRERENDER_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
    },
    { audience: "host" },
);

/** A stable cause of web failures. */
export type WebReason = ReasonOf<typeof WebError>;

import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The web package's failures, each reason with the call error code it answers. */
export const WebError = defineErrorFamily(
    import.meta.destack.package,
    {
        // bad request
        PAGE_DUPLICATE: { code: "BAD_REQUEST" },
        FEED_EMPTY: { code: "BAD_REQUEST" },
        FEED_DATE_INVALID: { code: "BAD_REQUEST" },
        OPTIONS_INVALID: { code: "BAD_REQUEST" },
        SECURITY_EXPIRY_INVALID: { code: "BAD_REQUEST" },
        ROUTE_NOT_CANONICAL: { code: "BAD_REQUEST" },
        SITE_REQUIRED: { code: "BAD_REQUEST" },
        SSR_REQUIRED: { code: "BAD_REQUEST" },

        // unprocessable content
        DIAGNOSTICS_TOO_LONG: { code: "UNPROCESSABLE_CONTENT" },
        PERSONALIZED_RESPONSE: { code: "UNPROCESSABLE_CONTENT" },
        PRERENDER_EXITED: { code: "UNPROCESSABLE_CONTENT" },
        PRERENDER_UNREADABLE: { code: "UNPROCESSABLE_CONTENT" },
        RESPONSE_UNEXPECTED: { code: "UNPROCESSABLE_CONTENT" },

        // gateway timeout
        PRERENDER_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
    },
    { audience: "host" },
);

/** A stable cause of web failures. */
export type WebReason = ReasonOf<typeof WebError>;

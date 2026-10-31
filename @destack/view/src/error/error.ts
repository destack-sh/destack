import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The view package's failures, each reason with the call error code it answers. */
export const ViewError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    FEED_DATE_INVALID: { code: "BAD_REQUEST" },
    FEED_EMPTY: { code: "BAD_REQUEST" },
    PAGE_DUPLICATE: { code: "BAD_REQUEST" },
    QUERY_VALUE_INVALID: { code: "BAD_REQUEST" },
    ROUTE_NOT_CANONICAL: { code: "BAD_REQUEST" },
    VIEW_NOT_STATIC: { code: "BAD_REQUEST" },
    SECURITY_LIFETIME_INVALID: { code: "BAD_REQUEST" },

    // forbidden
    NOTIFICATION_NOT_PERMITTED: { code: "FORBIDDEN" },

    // precondition failed
    GEOLOCATION_UNSUPPORTED: { code: "PRECONDITION_FAILED" },
    PACKAGE_NOT_DECLARED: { code: "PRECONDITION_FAILED" },
    SCOPE_NOT_OPENED: { code: "PRECONDITION_FAILED" },
    SHARE_UNSUPPORTED: { code: "PRECONDITION_FAILED" },

    // unprocessable content
    VIEW_DUPLICATE: { code: "UNPROCESSABLE_CONTENT" },
    RENDER_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT" },
    ISLAND_INVALID: { code: "UNPROCESSABLE_CONTENT" },
    ISLAND_PROPERTIES_INVALID: { code: "UNPROCESSABLE_CONTENT" },
    ISLAND_UNKNOWN: { code: "UNPROCESSABLE_CONTENT" },
    EXAMPLE_EMPTY: { code: "UNPROCESSABLE_CONTENT" },
    FONT_FILE_MISSING: { code: "UNPROCESSABLE_CONTENT" },
    FONT_FORMAT_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT" },
    IMAGE_NOT_RASTER: { code: "UNPROCESSABLE_CONTENT" },
    SOURCE_INVALID: { code: "UNPROCESSABLE_CONTENT" },
    MULTIPLE_THEMES: { code: "UNPROCESSABLE_CONTENT" },
    OBJECT_NOT_OPENED: { code: "UNPROCESSABLE_CONTENT" },
    PRESENTED_TYPE_NOT_OPENED: { code: "UNPROCESSABLE_CONTENT" },
    SERVER_FUNCTION_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT" },
    SETTING_SCOPE_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT" },
    RUNTIME_REQUIRED: { code: "UNPROCESSABLE_CONTENT" },
    PAGE_HEAD_MISSING: { code: "UNPROCESSABLE_CONTENT" },
    DIAGNOSTICS_TOO_LONG: { code: "UNPROCESSABLE_CONTENT" },
    PERSONALIZED_RESPONSE: { code: "UNPROCESSABLE_CONTENT" },
    PRERENDER_EXITED: { code: "UNPROCESSABLE_CONTENT" },
    PRERENDER_UNREADABLE: { code: "UNPROCESSABLE_CONTENT" },
    RESPONSE_UNEXPECTED: { code: "UNPROCESSABLE_CONTENT" },

    // internal server error
    PAGE_ADDRESS_UNKNOWN: { code: "INTERNAL_SERVER_ERROR" },
    PAGE_CONTEXT_MISSING: { code: "INTERNAL_SERVER_ERROR" },

    // bad gateway
    CATALOG_MISMATCH: { code: "BAD_GATEWAY" },
    CATALOG_UNAVAILABLE: { code: "BAD_GATEWAY" },
    FAVICON_LOAD_FAILED: { code: "BAD_GATEWAY" },
    UPLOAD_FAILED: { code: "BAD_GATEWAY" },
    UPLOAD_UNREACHABLE: { code: "BAD_GATEWAY" },

    // service unavailable
    GEOLOCATION_FAILED: { code: "SERVICE_UNAVAILABLE" },

    // gateway timeout
    PRERENDER_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
});

/** A stable cause of view failures. */
export type ViewReason = ReasonOf<typeof ViewError>;

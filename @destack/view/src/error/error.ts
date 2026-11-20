import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The view package's failures, each reason with the call error code it answers. */
export const ViewError = defineErrorFamily({
    reasons: {
        // invalid
        DATE_INVALID: {},
        DIAGNOSTICS_INVALID: {},
        FONT_FORMAT_INVALID: {},
        IMAGE_INVALID: {},
        ISLAND_INVALID: {},
        ISLAND_PROPERTIES_INVALID: {},
        MULTIPLE_THEMES_INVALID: {},
        OBJECT_INVALID: {},
        PAGE_INVALID: {},
        PRERENDER_INVALID: {},
        QUERY_VALUE_INVALID: {},
        RENDER_INVALID: {},
        RESPONSE_INVALID: {},
        ROUTE_INVALID: {},
        SECURITY_LIFETIME_INVALID: {},
        SERVER_FUNCTION_INVALID: {},
        SETTING_SCOPE_INVALID: {},
        SOURCE_INVALID: {},
        TYPE_INVALID: {},
        VIEW_INVALID: {},

        // required
        EXAMPLE_REQUIRED: {},
        FEED_REQUIRED: {},
        FONT_FILE_REQUIRED: {},
        PAGE_HEAD_REQUIRED: {},
        RUNTIME_REQUIRED: {},

        // denied
        NOTIFICATION_DENIED: {},

        // blocked
        PACKAGE_BLOCKED: {},
        SCOPE_BLOCKED: {},

        // unsupported
        GEOLOCATION_UNSUPPORTED: {},
        SHARE_UNSUPPORTED: {},

        // unavailable
        GEOLOCATION_UNAVAILABLE: {},

        // unreachable
        CATALOG_UNREACHABLE: {},
        FAVICON_LOAD_UNREACHABLE: {},
        PRERENDER_UNREACHABLE: {},
        UPLOAD_UNREACHABLE: {},

        // broken
        PAGE_ADDRESS_BROKEN: {},
        PAGE_CONTEXT_BROKEN: {},
    },
});

/** A stable cause of view failures. */
export type ViewReason = ReasonOf<typeof ViewError>;

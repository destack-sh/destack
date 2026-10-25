import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package owning the browser's failures, named as configurations load it unbuilt. */
const OWNER = { name: "@destack/environment" };

/** The browser environment's failures, each reason with the call error code it answers. */
export const BrowserError = defineErrorFamily(
    OWNER,
    {
        // precondition failed
        CAPABILITY_UNAVAILABLE: { code: "PRECONDITION_FAILED" },
    },
    { audience: "host" },
);

/** A stable cause of browser environment failures. */
export type BrowserReason = ReasonOf<typeof BrowserError>;

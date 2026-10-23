import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The fs package's failures, each reason with the call error code it answers. */
export const FileSystemError = defineErrorFamily(import.meta.destack.package, {
    // internal server error
    FILE_SYSTEM_FAILED: { code: "INTERNAL_SERVER_ERROR" },
});

/** A stable cause of fs failures. */
export type FileSystemReason = ReasonOf<typeof FileSystemError>;

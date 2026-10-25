import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package owning the file system's failures, named here as configurations load it unbuilt. */
const OWNER = { name: "@destack/environment" };

/** The file system's failures, each reason with the call error code it answers. */
const family = defineErrorFamily(
    OWNER,
    {
        // not found
        NOT_FOUND: { code: "NOT_FOUND" },

        // forbidden
        PERMISSION_DENIED: { code: "FORBIDDEN" },

        // conflict
        ALREADY_EXISTS: { code: "CONFLICT" },

        // internal server error
        FILE_SYSTEM_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    },
    { audience: "host" },
);

/** The file system's failures, each reason with the call error code it answers. */
export const FileSystemError = Object.assign(family, {
    /** Read a missing path as absent, rethrowing every other failure, as a read's `catch`. */
    absent: (error: unknown): undefined => {
        if (!family.is(error, "NOT_FOUND")) {
            throw error;
        }

        return undefined;
    },
});

/** A stable cause of file system failures. */
export type FileSystemReason = ReasonOf<typeof FileSystemError>;

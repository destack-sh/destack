import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package owning the file system's failures, named here as configurations load it unbuilt. */
const OWNER = { name: "@destack/environment" };

/** The file system's failures, each reason with the call error code it answers. */
const family = defineErrorFamily(
    {
        reasons: {
            // not found
            FILE_NOT_FOUND: {},

            // exists
            FILE_EXISTS: {},

            // denied
            PERMISSION_DENIED: {},

            // broken
            FILE_SYSTEM_BROKEN: {},
        },
        audience: "host",
    },
    { package: OWNER },
);

/** The file system's failures, each reason with the call error code it answers. */
export const FileSystemError = Object.assign(family, {
    /** Refuse an action by a POSIX error code such as `ENOENT`, caused by the host's failure. */
    posix: (code: string, action: string, cause?: unknown) => {
        const message = `cannot ${action}: ${code}`;
        const options = cause === undefined ? {} : { cause };
        if (code === "ENOENT" || code === "ENOTDIR") {
            return family.of("FILE_NOT_FOUND", message, options);
        } else if (code === "EEXIST" || code === "ENOTEMPTY") {
            return family.of("FILE_EXISTS", message, options);
        } else if (code === "EACCES" || code === "EPERM") {
            return family.of("PERMISSION_DENIED", message, options);
        } else {
            return family.of("FILE_SYSTEM_BROKEN", message, options);
        }
    },

    /** Read a missing path as absent, rethrowing every other failure, as a read's `catch`. */
    absent: (error: unknown): undefined => {
        if (!family.is(error, "FILE_NOT_FOUND")) {
            throw error;
        }

        return undefined;
    },
});

/** A stable cause of file system failures. */
export type FileSystemReason = ReasonOf<typeof FileSystemError>;

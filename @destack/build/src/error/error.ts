import {
    CallErrorCode,
    DomainError,
    defineErrorFamily,
    FamilyError,
    type ReasonOf,
} from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** The errors a Rolldown bundle failure aggregates. */
const BundleErrors = schema.array(schema.looseObject({ message: schema.string() })).min(1);

/** The build package's failures, each reason with the call error code it answers. */
export const BuildError = defineErrorFamily(
    import.meta.destack.package,
    {
        // bad request
        CACHE_DIRECTORY_INVALID: { code: "BAD_REQUEST" },
        DIGEST_INVALID: { code: "BAD_REQUEST" },
        MANIFEST_BLOB_UNKNOWN: { code: "BAD_REQUEST" },
        MANIFEST_INVALID: { code: "BAD_REQUEST" },
        MEDIA_TYPE_MISSING: { code: "BAD_REQUEST" },
        OUTPUT_INVALID: { code: "BAD_REQUEST" },
        TIMEOUT_INVALID: { code: "BAD_REQUEST" },

        // not found
        BLOB_UNKNOWN: { code: "NOT_FOUND" },
        MANIFEST_UNKNOWN: { code: "NOT_FOUND" },
        MEMBER_UNKNOWN: { code: "NOT_FOUND" },

        // conflict
        SOURCE_CHANGED: { code: "CONFLICT" },

        // unprocessable content
        ASSET_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        BUNDLE_FAILED: { code: "UNPROCESSABLE_CONTENT" },
        CATALOG_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        DECLARATION_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        DECLARATION_UNEXPORTED: { code: "UNPROCESSABLE_CONTENT" },
        DEPENDENCY_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        DEPENDENCY_UNRESOLVED: { code: "UNPROCESSABLE_CONTENT" },
        ENTRY_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        EXTENSION_FAILED: { code: "UNPROCESSABLE_CONTENT" },
        EXTENSION_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        INSPECTION_FAILED: { code: "UNPROCESSABLE_CONTENT" },
        LOCKFILE_INVALID: { code: "UNPROCESSABLE_CONTENT" },
        RUNTIME_API_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT" },
        RUNTIME_MODULE_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT" },

        // client closed request
        BUILD_CANCELLED: { code: "CLIENT_CLOSED_REQUEST" },

        // internal server error
        BUILD_NOT_IN_BUCKET: { code: "INTERNAL_SERVER_ERROR" },
        CHECKOUT_FAILED: { code: "INTERNAL_SERVER_ERROR" },
        COMPILER_FAILED: { code: "INTERNAL_SERVER_ERROR" },
        INVARIANT_VIOLATED: { code: "INTERNAL_SERVER_ERROR" },
        STORED_CONTENT_DIFFERS: { code: "INTERNAL_SERVER_ERROR" },

        // bad gateway
        DIGEST_MISMATCH: { code: "BAD_GATEWAY" },
        PROBE_FAILED: { code: "BAD_GATEWAY" },
        UPLOAD_FAILED: { code: "BAD_GATEWAY" },

        // gateway timeout
        COMPILER_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
    },
    { audience: "host" },
);

/** A stable cause of build failures. */
export type BuildReason = ReasonOf<typeof BuildError>;

/** Report a build extension's failure, keeping a package's own failure and a bundle failure. */
export function extensionFailureOf(cause: unknown): Error {
    // keep a package's declared failure, such as a web or view refusal, as it is
    if (DomainError.is(cause)) {
        return cause;
    }
    // report the bundle failure of a compiling extension
    else if (isBundleFailure(cause)) {
        return bundleFailureOf(cause);
    }
    // report any other failure as the extension's
    else {
        const message = cause instanceof Error ? cause.message : String(cause);

        return BuildError.of("EXTENSION_FAILED", message, { cause });
    }
}

/** Report a failed bundle by its error messages, keeping a plugin's family failure as it is. */
export function bundleFailureOf(error: unknown): Error {
    // find a family failure a plugin threw among the errors a Rolldown bundle failure aggregates
    const errors: unknown = error instanceof Error ? Reflect.get(error, "errors") : undefined;
    const entries: unknown[] = Array.isArray(errors) ? errors : [];
    const declared = entries.find((entry): entry is FamilyError => entry instanceof FamilyError);

    // restore its code, which Rolldown moves to pluginCode when it marks a hook's error
    if (declared !== undefined) {
        const code: unknown = Reflect.get(declared, "pluginCode") ?? declared.code;
        if (typeof code !== "string" || !CallErrorCode.is(code)) {
            return BuildError.of(
                "INVARIANT_VIOLATED",
                `the bundler lost the code of ${declared.reason}`,
                {
                    cause: error,
                },
            );
        }
        const { domain, reason, data, audience } = declared;

        return new FamilyError({ domain, reason, code, data, audience }, declared.message, {
            cause: declared.cause,
        });
    }

    // read the errors a Rolldown bundle failure aggregates
    const parsed = BundleErrors.safeParse(errors);
    if (!parsed.success) {
        const message = error instanceof Error ? error.message : String(error);

        return BuildError.of("BUNDLE_FAILED", message, { cause: error });
    }
    const messages = parsed.data.map((entry) => entry.message);

    return BuildError.of("BUNDLE_FAILED", messages.join("\n"), { cause: error });
}

/** Report whether a failure is a Rolldown bundle failure, which aggregates its errors. */
function isBundleFailure(error: unknown): boolean {
    return error instanceof Error && BundleErrors.safeParse(Reflect.get(error, "errors")).success;
}

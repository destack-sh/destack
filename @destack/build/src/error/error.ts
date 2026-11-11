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
export const BuildError = defineErrorFamily({
    reasons: {
        // not found
        BLOB_NOT_FOUND: {},
        MANIFEST_NOT_FOUND: {},
        MEMBER_NOT_FOUND: {},

        // conflict
        SOURCE_CONFLICT: {},

        // invalid
        ASSET_INVALID: {},
        BUNDLE_INVALID: {},
        CACHE_DIRECTORY_INVALID: {},
        CATALOG_INVALID: {},
        DECLARATION_INVALID: {},
        DEPENDENCY_INVALID: {},
        DIGEST_INVALID: {},
        ENTRY_INVALID: {},
        EXTENSION_INVALID: {},
        INSPECTION_INVALID: {},
        LOCKFILE_INVALID: {},
        MANIFEST_BLOB_INVALID: {},
        MANIFEST_INVALID: {},
        OUTPUT_INVALID: {},
        RUNTIME_API_INVALID: {},
        RUNTIME_MODULE_INVALID: {},
        TIMEOUT_INVALID: {},

        // required
        MEDIA_TYPE_REQUIRED: {},

        // unreachable
        COMPILER_UNREACHABLE: {},
        DIGEST_UNREACHABLE: {},
        PROBE_UNREACHABLE: {},
        UPLOAD_UNREACHABLE: {},

        // cancelled
        BUILD_CANCELLED: {},

        // broken
        CHECKOUT_BROKEN: {},
        COMPILER_BROKEN: {},
        CONTENT_BROKEN: {},
        INVARIANT_BROKEN: {},
        LAYER_FILE_BROKEN: {},
    },
    audience: "host",
});

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

        return BuildError.of("EXTENSION_INVALID", message, { cause });
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
                "INVARIANT_BROKEN",
                `the bundler lost the code of '${declared.reason}'`,
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

        return BuildError.of("BUNDLE_INVALID", message, { cause: error });
    }
    const messages = parsed.data.map((entry) => entry.message);

    return BuildError.of("BUNDLE_INVALID", messages.join("\n"), { cause: error });
}

/** Report whether a failure is a Rolldown bundle failure, which aggregates its errors. */
function isBundleFailure(error: unknown): boolean {
    return error instanceof Error && BundleErrors.safeParse(Reflect.get(error, "errors")).success;
}

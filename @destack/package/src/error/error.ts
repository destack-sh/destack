import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import { SandboxName } from "../definition/sandbox.ts";

/** The package name: the Bun preload loads this module before metadata injection exists. */
const DOMAIN = "@destack/package";

/** The sandbox ability a failure concerns, such as network or fs. */
export const SandboxAbility = schema.object({
    /** The ability's name, read lazily since the sandbox declaration imports this module. */
    ability: schema.lazy(() => SandboxName),
});
/** The sandbox ability a failure concerns. */
export type SandboxAbility = schema.Infer<typeof SandboxAbility>;

/** The package package's failures, each reason with the call error code it answers. */
export const PackageError = defineErrorFamily(
    { name: DOMAIN },
    {
        // bad request
        INVALID_BUILD: { code: "BAD_REQUEST" },
        INVALID_DEFINITION: { code: "BAD_REQUEST" },
        INVALID_DEPENDENCY: { code: "BAD_REQUEST" },
        INVALID_EXPORT: { code: "BAD_REQUEST" },
        INVALID_FILE: { code: "BAD_REQUEST" },

        // unprocessable content
        ABILITY_UNENFORCEABLE: { code: "UNPROCESSABLE_CONTENT", data: SandboxAbility },
        ABILITY_UNSUPPORTED: { code: "UNPROCESSABLE_CONTENT", data: SandboxAbility },
        DUPLICATE_VIEW: { code: "UNPROCESSABLE_CONTENT" },
        HOST_MODULE_UNAVAILABLE: { code: "UNPROCESSABLE_CONTENT" },
        INVALID_INSPECTION: { code: "UNPROCESSABLE_CONTENT" },
        UNSUPPORTED_LANGUAGE: { code: "UNPROCESSABLE_CONTENT" },
        UNSUPPORTED_TARGET: { code: "UNPROCESSABLE_CONTENT" },
    },
);

/** A stable cause of package failures. */
export type PackageReason = ReasonOf<typeof PackageError>;

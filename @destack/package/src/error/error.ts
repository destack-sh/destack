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
        // invalid
        ABILITY_INVALID: { data: SandboxAbility },
        BUILD_INVALID: {},
        DEFINITION_INVALID: {},
        DEPENDENCY_INVALID: {},
        EXPORT_INVALID: {},
        FILE_INVALID: {},
        HOST_MODULE_INVALID: {},
        INSPECTION_INVALID: {},
        LANGUAGE_INVALID: {},
        TARGET_INVALID: {},
        VIEW_INVALID: {},
        VIEW_MAIN_INVALID: {},

        // unavailable
        FILE_UNAVAILABLE: {},
    },
);

/** A stable cause of package failures. */
export type PackageReason = ReasonOf<typeof PackageError>;

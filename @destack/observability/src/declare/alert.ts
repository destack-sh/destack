import { ModuleMetadata, type Package } from "@destack/package";
import type { schema } from "@destack/schema";
import { AlertRuleDefinition } from "../object/alert-rule.ts";

/** An alert rule a package declares, which each installation of the package keeps in its space. */
export interface AlertRuleDeclaration {
    /** The declaring package. */
    readonly package: Package;
    /** The rule. */
    readonly definition: AlertRuleDefinition;
}

/** Declare an alert rule each installation of the package keeps in its space, such as rolling back new fatal issues. */
export function defineAlertRule(
    definition: schema.Input<typeof AlertRuleDefinition>,
    module?: ModuleMetadata,
): AlertRuleDeclaration {
    return {
        package: ModuleMetadata.require(module, "defineAlertRule").package,
        definition: AlertRuleDefinition.parse(definition),
    };
}

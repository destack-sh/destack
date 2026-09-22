import type { Rule } from "@oxlint/plugins";
import { errorMessageStyle } from "./error-message-style.ts";
import { noInlineConfig } from "./no-inline-config.ts";

/** A trusted Oxc plugin selected by the host's dependency resolver. */
export interface Plugin {
    /** Unique rule namespace. */
    name: string;
    /** Absolute module path for managed checks, or package export for editor configuration. */
    specifier: string;
    /** Rules enabled as errors whenever this plugin is selected. */
    rules: Record<string, Rule>;
}

/** Mandatory Destack source rules. */
const plugin = {
    meta: { name: "destack" },
    rules: { "error-message-style": errorMessageStyle, "no-inline-config": noInlineConfig },
};

export default plugin;

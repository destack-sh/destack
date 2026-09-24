import { PackageId, type Package } from "@destack/package";
import { reference } from "@destack/package/declare";
import { defineSchema, schema } from "@destack/schema";
import type { SettingDefinition } from "../declare/setting.ts";
import type { SettingContext } from "./context.ts";
import type { SettingResolution } from "./resolution.ts";
import type { SettingTarget } from "./target.ts";
import { SettingError } from "../error/error.ts";

/** A package-local setting name, retained across releases. */
export const SettingName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/),
);

/** The stable identity of a setting across package renames and releases. */
export const SettingReference = defineSchema(
    schema.object({
        /** The package defining the setting. */
        packageId: PackageId,
        /** The stable declaration name. */
        name: SettingName,
    }),
);
/** The stable identity of a setting. */
export type SettingReference = schema.Infer<typeof SettingReference>;

/** A typed setting declaration with invocation-scoped access. */
export class Setting<Value extends schema.Schema = schema.Schema> {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The package-local setting name. */
    readonly name: string;
    /** The schema, default and supported application scopes. */
    readonly definition: SettingDefinition<Value>;
    /** The stable identity used by assignments and policies. */
    readonly reference: SettingReference;

    /** Retain a checked declaration without loading assignments. */
    constructor(owner: Package, definition: SettingDefinition<Value>) {
        // retain the declaring package, name and definition
        this.package = owner;
        this.name = definition.name;
        this.definition = definition;
        this.reference = reference(this);
    }

    /** Read the effective value for the host-selected context. */
    async get(context: SettingContext): Promise<schema.Infer<Value>> {
        const resolution = await this.resolve(context);

        return resolution.value;
    }

    /** Explain the effective value for the host-selected context. */
    async resolve(context: SettingContext): Promise<SettingResolution<schema.Infer<Value>>> {
        const result = await context.resolve({ setting: this });

        return result.setting;
    }

    /** Require an assignment to use the declared scope and supported refinements. */
    assertTarget(target: SettingTarget): void {
        // require the declaration's base scope before considering refinements
        if (target.kind !== this.definition.scope) {
            throw new SettingError(
                "INVALID_TARGET",
                "assignment scope does not match the setting declaration",
            );
        }

        // select refinements independently of the user or host's current identity
        const refinements: string[] = [];
        if (target.kind === "user" && target.packageId) {
            refinements.push("package");
        }
        if ("location" in target && target.location) {
            if (target.location.installationId) {
                refinements.push("installation");
            } else if (target.kind === "user") {
                refinements.push("space");
            }
        }
        if (target.kind === "user" && target.deviceId) {
            refinements.push("device");
        }

        // apply the same restrictions to source declarations, writes and retained assignments
        const overrides: readonly string[] = this.definition.overrides;
        if (refinements.some((refinement) => !overrides.includes(refinement))) {
            throw new SettingError(
                "INVALID_TARGET",
                "assignment uses an unsupported setting override",
            );
        }
    }
}

import { ModuleMetadata, type Package } from "@destack/package";
import { defineSchema, type JsonObject, JsonValue, schema } from "@destack/schema";
import { InstallationDefinition } from "./installation.ts";
import { ResourceDefinition } from "@destack/resource";
import type { RelationshipDefinition, RoleDefinition } from "./permission.ts";
import { RouteDefinition } from "./route.ts";
import { type Setting, SettingReference } from "@destack/setting";
import type { StackSetting } from "@destack/setting/declare";

/** The collections a stack declares, extended by module augmentation in object packages. */
export interface StackDefinition {
    /** Resources created or adopted by the stack. */
    readonly resources?: Readonly<Record<string, schema.Input<typeof ResourceDefinition>>>;
    /** Independently configured package installations. */
    readonly installations?: Readonly<Record<string, schema.Input<typeof InstallationDefinition>>>;
    /** Roles defined in the space. */
    readonly roles?: Readonly<Record<string, schema.Input<typeof RoleDefinition>>>;
    /** Roles bound and relations granted to members, groups and workloads. */
    readonly relationships?: Readonly<Record<string, schema.Input<typeof RelationshipDefinition>>>;
    /** Routes under domains the space's account administers. */
    readonly routes?: Readonly<Record<string, schema.Input<typeof RouteDefinition>>>;
    /** The setting values the stack places in its space. */
    readonly settings?: Readonly<
        Record<
            string,
            Omit<StackSetting, "setting"> & { readonly setting: Setting | SettingReference }
        >
    >;
}

/** The collections of a stack definition the stack package reads itself. */
const StackCollections = schema.looseObject({
    /** Resources created or adopted by the stack. */
    resources: schema.record(schema.string(), ResourceDefinition).default({}),
    /** Independently configured package installations. */
    installations: schema.record(schema.string(), InstallationDefinition).default({}),
    /** The setting values the stack places, by key. */
    settings: schema.jsonObject().default({}),
});

/** A stack definition as JSON, keyed by collection and validated by each object when applied. */
export const StackDefinition = Object.assign(defineSchema(schema.jsonObject()), {
    /** Read the collections the stack package applies itself. */
    collections(definition: Readonly<Record<string, unknown>>) {
        return StackCollections.parse(definition);
    },

    /** Describe the setting values a space carries as the settings a stack places. */
    placed(settings: Readonly<Record<string, JsonValue>>): Record<string, JsonObject> {
        return Object.fromEntries(
            Object.entries(settings).map(([key, value]) => [
                key,
                { setting: SettingReference.of(key), mode: "set", value },
            ]),
        );
    },
});

/** A stack's declarations with the package declaring them. */
export interface StackDeclaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The definition as JSON, keyed by collection. */
    readonly definition: JsonObject;
}

/** Define a stack's declarations, serialising each declared value to its JSON form. */
export function defineStack(
    definition: StackDefinition,
    module?: ModuleMetadata,
): StackDeclaration {
    return {
        package: ModuleMetadata.require(module, "defineStack").package,
        definition: StackDefinition.parse(JsonValue.of(definition)),
    };
}

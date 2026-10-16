import {
    defineSchema,
    fromJsonSchema,
    JsonValue,
    schema,
    toJsonSchema,
    Version,
} from "@destack/schema";
import { Expression } from "@destack/db";
import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { BuildCache, type BuildReader } from "@destack/package/manifest";
import { SettingMetadata, SettingScope } from "../declare/setting.ts";
import { SettingError } from "../error/error.ts";
import type { SettingPlacement } from "./placement.ts";
import { Setting, SettingReference, type SettingWrite } from "./setting.ts";

/** The serializable fields of a setting description beside its scope. */
const description = SettingMetadata.extend({
    /** The declaring package. */
    package: Package,
    /** The value's JSON Schema. */
    schema: schema.jsonObject(),
    /** The default value. */
    default: schema.json(),
    /** The conversions of earlier values, by the release introducing each. */
    convert: schema.record(Version, Expression.schema).exactOptional(),
});

/** A setting declaration as manifests describe it. */
export const SettingDescription = defineSchema(
    schema.discriminatedUnion("scope", [
        SettingScope.options[0].extend(description.shape),
        SettingScope.options[1].extend(description.shape),
        SettingScope.options[2].extend(description.shape),
    ]),
);
/** A setting declaration as manifests describe it. */
export type SettingDescription = schema.Infer<typeof SettingDescription>;

/** Describe a setting and its conversions, so every build reader converts earlier values. */
export function describeSetting(setting: Setting): SettingDescription {
    const { schema: valueSchema, ...definition } = setting.definition;

    return {
        ...definition,
        package: setting.package,
        default: schema.json().parse(definition.default),
        schema: JsonValue.object(toJsonSchema(valueSchema)),
    };
}

/** The settings a build declares, by key. */
export class SettingCatalog {
    /** The declared settings in declaration order. */
    readonly settings: readonly Setting[];
    /** The declared settings by key. */
    readonly #byKey: ReadonlyMap<string, Setting>;

    /** Hold a build's settings. */
    constructor(settings: readonly Setting[]) {
        this.settings = settings;
        this.#byKey = new Map(
            settings.map((setting) => [SettingReference.key(setting.reference), setting]),
        );
    }

    /** Read the settings a build declares. */
    static async read(reader: BuildReader): Promise<SettingCatalog> {
        const descriptions = await reader.declared(
            import.meta.destack.package.id,
            "setting",
            SettingDescription,
        );

        return SettingCatalog.of(descriptions.map((entry) => entry.description));
    }

    /** Collect the settings some descriptions declare, such as those of a space's installations. */
    static of(descriptions: readonly SettingDescription[]): SettingCatalog {
        return new SettingCatalog(
            descriptions.map(
                ({ package: owner, schema: valueSchema, ...definition }) =>
                    new Setting(owner, { ...definition, schema: fromJsonSchema(valueSchema) }),
            ),
        );
    }

    /** Read each written value's catalog from the build `open` opens for it. */
    static cached(
        open: (
            written: SettingReference & SettingPlacement & Pick<SettingWrite, "release">,
        ) => Promise<BuildReader>,
    ): (
        written: SettingReference & SettingPlacement & Pick<SettingWrite, "release">,
    ) => Promise<SettingCatalog> {
        const catalogs = new BuildCache((reader) => SettingCatalog.read(reader));

        return async (written) => catalogs.read(await open(written));
    }

    /** Find a declared setting and refuse an undeclared one. */
    get(reference: SettingReference): Setting {
        // look the setting up by its key
        const key = SettingReference.key(reference);
        const setting = this.#byKey.get(key);
        if (setting === undefined) {
            throw new SettingError("UNDECLARED", `setting ${key} is not declared`);
        }

        return setting;
    }
}

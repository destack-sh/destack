import { defineSchema, fromJsonSchema, schema, toJsonSchema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { BuildCache, type BuildReader } from "@destack/package/manifest";
import { SettingMetadata, SettingScope } from "../declare/setting.ts";
import { SettingError } from "../error/error.ts";
import type { SettingPlacement } from "./placement.ts";
import { Setting, SettingReference, type SettingWrite } from "./setting.ts";

/** Open the settings declared where a written value is checked: its release's, or the ones a process ships. */
export type OpenSettingCatalog = (
    written: SettingReference & SettingPlacement & Pick<SettingWrite, "release">,
) => Promise<SettingCatalog>;

/** Open the build declaring a written value's setting: the release its space installs, or the release the value names. */
export type OpenSettingRelease = (written: Parameters<OpenSettingCatalog>[0]) => Promise<BuildReader>;

/** The serializable fields of a setting description beside its scope. */
const description = SettingMetadata.extend({
    /** The declaring package. */
    package: Package,
    /** The value's JSON Schema. */
    schema: schema.record(schema.string(), schema.json()),
    /** The default value. */
    default: schema.json(),
    /** The value each release computes from a value of an earlier release, by the release introducing it. */
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

/** Describe a setting with its conversions, so every reader of the build converts earlier values. */
export function describeSetting(setting: Setting): SettingDescription {
    const { schema: valueSchema, ...definition } = setting.definition;

    return {
        ...definition,
        package: setting.package,
        default: schema.json().parse(definition.default),
        schema: schema.record(schema.string(), schema.json()).parse(toJsonSchema(valueSchema)),
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

    /** Hold the settings some descriptions declare, such as a space's installations' declarations. */
    static of(descriptions: readonly SettingDescription[]): SettingCatalog {
        return new SettingCatalog(
            descriptions.map(
                ({ package: owner, schema: valueSchema, ...definition }) =>
                    new Setting(owner, { ...definition, schema: fromJsonSchema(valueSchema) }),
            ),
        );
    }

    /** Open each written value's catalog from the build its release opens, reading each build's settings once. */
    static releases(open: OpenSettingRelease): OpenSettingCatalog {
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

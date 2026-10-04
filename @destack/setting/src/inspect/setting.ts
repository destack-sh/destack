import {
    type JsonValue,
    defineSchema,
    fromJsonSchema,
    schema,
    toJsonSchema,
    Version,
} from "@destack/schema";
import { Expression } from "@destack/db";
import { Address, Plan, type Comparator } from "@destack/resource";
import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";
import type { BuildReader } from "@destack/package/manifest";
import { SettingMetadata, SettingScope } from "../declare/setting.ts";
import { SettingError } from "../error/index.ts";
import { Setting, SettingReference } from "../setting/setting.ts";

/** The catalog read from each build so far. */
const READ = new WeakMap<BuildReader, Promise<SettingCatalog>>();

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

/** Plan a setting's value change between two releases: newer readers read earlier values, converted by a conversion of a release between them. */
export const compareSetting: Comparator = (before, after) => {
    // read both releases' settings and the releases declaring them
    const earlier = SettingDescription.parse(before.description);
    const later = SettingDescription.parse(after.description);
    const release = after.symbol.package.version;

    return Plan.values({
        target: Address.join("setting", later.name),
        before: earlier.schema,
        after: later.schema,
        release,
        compatibility: "backward",
        isConverted:
            Version.between(later.convert ?? {}, before.symbol.package.version, release).length > 0,
    });
};

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

    /** Read the settings a build declares once per build. */
    static read(reader: BuildReader): Promise<SettingCatalog> {
        // build each release's validators once
        let catalog = READ.get(reader);
        if (catalog === undefined) {
            catalog = reader
                .declared(import.meta.destack.package.id, "setting", SettingDescription)
                .then(
                    (descriptions) =>
                        new SettingCatalog(
                            descriptions.map(
                                ({
                                    description: {
                                        package: owner,
                                        schema: valueSchema,
                                        ...definition
                                    },
                                }) =>
                                    new Setting(owner, {
                                        ...definition,
                                        schema: fromJsonSchema(valueSchema),
                                    }),
                            ),
                        ),
                );
            READ.set(reader, catalog);

            // read again after a failed read
            catalog.catch(() => READ.delete(reader));
        }

        return catalog;
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

/** List a setting's term: its name, with its scope. */
export function settingVocabulary(input: Record<string, JsonValue>): Record<string, JsonValue> {
    const declared = SettingDescription.parse(input);

    return { [declared.name]: { scope: declared.scope } };
}

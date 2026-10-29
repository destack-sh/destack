import { DeclarationReference } from "@destack/package/declare";
import { Package, DeclarationName } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { DeclarationCache, type BuildReader } from "@destack/package/manifest";
import { fromJsonSchema, nth, schema, Version, canonicalize } from "@destack/schema";
import type { JsonObject, JsonValue } from "@destack/schema";
import { Expression, Condition, type Table } from "@destack/db";
import type { RelationalQueryBuilder } from "@destack/db";
import type { SettingDefault, SettingDefinition } from "../declare/setting.ts";
import type { SettingValue } from "../object/setting.ts";
import type { DeviceSettingValue } from "../object/device-setting.ts";
import type { SettingResolution, SettingSource } from "./resolution.ts";
import { SettingPlacement, type SettingSelection } from "./placement.ts";
import type { SettingMode } from "./mode.ts";
import { SettingError } from "../error/error.ts";

/** The declaration default ranks below every placed value. */
const DEFAULT_TIER = 0;
/** Recommendations of enclosing scopes follow the declaration default. */
const ENCLOSING_TIER = 1;
/** Values set in the selected scope follow every recommendation. */
const SET_TIER = 2;
/** Values the device stores follow every value set in the selected scope. */
const DEVICE_TIER = 3;

/** An enclosing recommendation one scope nearer follows a farther one, for any installation. */
const DEPTH_WEIGHT = 2;
/** A recommendation for the selected installation follows a broader one. */
const ENCLOSING_INSTALLATION_WEIGHT = 1;

/** A set value for the consuming package follows one applying to every package. */
const PACKAGE_WEIGHT = 1;
/** A set value for the space follows only the package override. */
const SPACE_WEIGHT = 2;
/** A set value for the installation follows one for its space. */
const INSTALLATION_WEIGHT = 4;

/** The scope a setting belongs to and the overrides it permits. */
export const SettingScope = schema.discriminatedUnion("scope", [
    schema.object({
        /** A value of each user. */
        scope: schema.literal("user"),
        /** The overrides the setting permits. */
        overrides: schema.array(schema.enum(["package", "space", "installation", "device"])),
    }),
    schema.object({
        /** A value of each space. */
        scope: schema.literal("space"),
        /** The overrides the setting permits. */
        overrides: schema.array(schema.literal("installation")),
    }),
    schema.object({
        /** A value each device stores for itself, as a local object. */
        scope: schema.literal("device"),
        /** The overrides the setting permits: none. */
        overrides: schema.tuple([]),
    }),
]);
/** The scope a setting belongs to and the overrides it permits. */
export type SettingScope = schema.Infer<typeof SettingScope>;

/** The serializable fields of a setting declaration. */
export const SettingMetadata = schema.object({
    /** The package-local name. */
    name: DeclarationName,
    /** The label settings views show. */
    title: schema.string().min(1),
    /** The behavior the value controls. */
    description: schema.string().min(1),
    /** The group settings views show it in. */
    group: schema.string().min(1).exactOptional(),
    /** When a consumer applies a changed value. */
    apply: schema.enum(["immediate", "restart"]),
    /** The migration guidance of a deprecated setting. */
    deprecated: schema.string().min(1).exactOptional(),
    /** How placed values combine. */
    merge: schema.enum(["value", "key"]).exactOptional(),
});

/** The serializable fields of a setting description beside its scope. */
const fields = SettingMetadata.extend({
    /** The declaring package. */
    package: Package,
    /** The value's JSON Schema. */
    schema: schema.jsonObject(),
    /** The default value, absent for a required setting. */
    default: schema.json().exactOptional(),
    /** Whether each scope must place a value, as the setting has no default. */
    required: schema.literal(true).exactOptional(),
    /** The conversions of earlier values, by the release introducing each. */
    convert: schema.record(Version, Expression.schema).exactOptional(),
});

/** A setting declaration as manifests describe it. */
export const SettingDescription = schema.discriminatedUnion("scope", [
    SettingScope.options[0].extend(fields.shape),
    SettingScope.options[1].extend(fields.shape),
    SettingScope.options[2].extend(fields.shape),
]);
/** A setting declaration as manifests describe it. */
export type SettingDescription = schema.Infer<typeof SettingDescription>;

/** A value written in a scope, from a write or a stack's declaration. */
export type SettingWrite = Omit<SettingPlacement, "scope"> & {
    /** How the value applies. */
    readonly mode: SettingMode;
    /** The value. */
    readonly value: JsonValue;
    /** The release of the setting's package the value was written against. */
    readonly release: Version;
};

/** A typed setting declaration, typed by whether it has a default. */
export class Setting<
    Value extends schema.Schema = schema.Schema,
    Default extends SettingDefault<Value> = SettingDefault<Value>,
> {
    /** The declaring package. */
    readonly package: Package;
    /** The name, schema, default, scopes and conversions. */
    readonly definition: SettingDefinition<Value, Default>;

    /** Create a setting from a checked declaration. */
    constructor(owner: Package, definition: SettingDefinition<Value, Default>) {
        this.package = owner;
        this.definition = definition;
    }

    /** Rebuild a setting from its description, its schema from its JSON Schema. */
    static of(description: SettingDescription): Setting {
        // require exactly one of a default and a requirement
        const {
            package: owner,
            schema: valueSchema,
            default: given,
            required,
            ...definition
        } = description;
        if ((given === undefined) !== (required === true)) {
            throw new TypeError(`setting ${definition.name} needs either a default or required`);
        }
        const defaulted = given === undefined ? { required: true as const } : { default: given };

        return new Setting(owner, {
            ...definition,
            ...defaulted,
            schema: fromJsonSchema(valueSchema),
        });
    }

    /** Read the settings a build declares, in declaration order. */
    static async list(reader: BuildReader): Promise<Setting[]> {
        const declared = await reader.declared(
            import.meta.destack.package.id,
            "setting",
            SettingDescription,
        );

        return declared.map((entry) => Setting.of(entry.description));
    }

    /** Find a declared setting and refuse an undeclared one. */
    static find(settings: readonly Setting[], reference: DeclarationReference): Setting {
        // look the setting up by its key
        const key = DeclarationReference.key(reference);
        const setting = settings.find((each) => DeclarationReference.key(each.reference) === key);
        if (setting === undefined) {
            throw SettingError.of("SETTING_UNDECLARED", `setting ${key} is not declared`);
        }

        return setting;
    }

    /** Find a value's setting in the build `open` opens for it in a view, each build read once. */
    static declared(
        open: (written: SettingDeclared, view: SettingView) => Promise<BuildReader>,
    ): (written: SettingDeclared, view: SettingView) => Promise<Setting> {
        const lists = new DeclarationCache((reader) => Setting.list(reader));

        return async (written, view) =>
            Setting.find(await lists.read(await open(written, view)), written);
    }

    /** The identity its values refer to. */
    get reference(): DeclarationReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Read the setting's value among an installation's resolved settings. */
    read(settings: JsonObject): schema.Infer<Value> {
        // require the value its machine resolved
        const key = DeclarationReference.key(this.reference);
        if (!Object.hasOwn(settings, key)) {
            throw SettingError.of(
                "SETTING_UNDECLARED",
                `the installation's machine did not resolve ${key}`,
            );
        }

        return this.definition.schema.parse(settings[key]);
    }

    /** Serialise the setting as its reference. */
    toJSON(): DeclarationReference {
        return this.reference;
    }

    /** Match the values a resolution reads. */
    condition(selection: SettingSelection): Condition {
        const policy = { mode: { ne: "set" } };

        return {
            AND: [
                { packageId: this.reference.packageId },
                { name: this.reference.name },
                selection.scope === null ? policy : { OR: [{ scope: selection.scope }, policy] },
            ],
        };
    }

    /** Resolve the effective value for a selection, skipping invalid stored values as sources. */
    resolve(
        selection: SettingSelection,
        values: readonly SettingValue[],
        chain: readonly string[],
        device: readonly DeviceSettingValue[] = [],
    ): SettingResolution<schema.Infer<Value>> {
        // rank the default and each applicable placed and device value of a selection of the setting's scope
        this.#requireSelectable(selection);
        const { ordinary, enforced, invalid } = this.#candidates(values, selection, chain, device);

        // refuse a setting without a default that no scope placed, or placed only invalid values
        const key = DeclarationReference.key(this.reference);
        if (ordinary.length === 0 && enforced.length === 0 && invalid.length === 0) {
            throw SettingError.of("SETTING_UNSET", `setting ${key} has no default and is not set`);
        } else if (ordinary.length === 0 && enforced.length === 0) {
            throw SettingError.of(
                "SETTING_INVALID",
                `setting ${key} has no default and no valid value`,
            );
        }

        // combine each key from its own nearest placement for a setting merging keys, or take one value
        return this.definition.merge === "key"
            ? this.#mergeKeys(selection, ordinary, enforced, invalid)
            : this.#decide(selection, ordinary, enforced, invalid);
    }

    /** Report whether devices store values of this setting. */
    isDeviceStored(): boolean {
        const { scope, overrides } = this.definition;

        return scope === "device" || overrides.some((override) => override === "device");
    }

    /** Require a selection of the setting's scope, or an anonymous one of a user setting. */
    #requireSelectable(selection: SettingSelection): void {
        const isSelectable =
            selection.scope === null
                ? this.definition.scope === "user"
                : schema.identifier(this.definition.scope).safeParse(selection.scope).success;
        if (!isSelectable) {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                "setting scope does not match the selected scope",
            );
        }
    }

    /** Rank the default and each applicable placed and device value, invalid ones apart. */
    #candidates(
        values: readonly SettingValue[],
        selection: SettingSelection,
        chain: readonly string[],
        device: readonly DeviceSettingValue[],
    ): {
        readonly ordinary: Candidate[];
        readonly enforced: Candidate[];
        readonly invalid: SettingSource[];
    } {
        // collect the default of a setting with one, and each applicable value apart from invalid ones
        const { definition } = this;
        const ordinary: Candidate[] =
            "default" in definition
                ? [
                      {
                          rank: [DEFAULT_TIER, 0],
                          source: { kind: "default", package: this.package },
                          value: definition.default,
                          mode: "set",
                      },
                  ]
                : [];
        const enforced: Candidate[] = [];
        const invalid: SettingSource[] = [];
        const candidates = [
            ...values.map((value) => this.#candidate(value, selection, chain)),
            ...device.map((value) => this.#deviceCandidate(value)),
        ].filter((candidate) => candidate !== undefined);
        for (const candidate of candidates) {
            // an invalid value, set apart
            if (candidate.source.kind === "invalid") {
                invalid.push(candidate.source);
            }
            // a requirement or an ordinary value
            else {
                (candidate.mode === "require" ? enforced : ordinary).push(candidate);
            }
        }

        // order candidates independently of arrival order
        ordinary.sort((left, right) => compareRanks(left, right) || compareSources(left, right));
        enforced.sort(compareSources);
        invalid.sort((left, right) => compareCanonical(left, right));

        return { ordinary, enforced, invalid };
    }

    /** Decide one value: agreeing requirements, or else the highest-ranked candidates. */
    #decide(
        selection: SettingSelection,
        ordinary: readonly Candidate[],
        enforced: readonly Candidate[],
        invalid: readonly SettingSource[],
    ): SettingResolution<schema.Infer<Value>> {
        // reject equally ranked candidates other than agreeing recommendations
        for (let index = 1; index < ordinary.length; index++) {
            const previous = nth(ordinary, index - 1);
            const current = nth(ordinary, index);
            const isShared =
                previous.mode === "recommend" &&
                current.mode === "recommend" &&
                equalValue(previous.value, current.value);
            if (compareRanks(previous, current) === 0 && !isShared) {
                throw SettingError.of(
                    "VALUES_CONFLICTING",
                    "multiple setting values have the same precedence",
                );
            }
        }

        // reject disagreeing requirements
        const [requirement] = enforced;
        if (
            requirement !== undefined &&
            enforced.some((candidate) => !equalValue(candidate.value, requirement.value))
        ) {
            throw SettingError.of("VALUES_CONFLICTING", "required setting values conflict");
        }

        // take the requirements, or else the highest ranked candidates
        const winner = requirement ?? nth(ordinary, -1);
        const winners =
            enforced.length > 0
                ? enforced
                : ordinary.filter((candidate) => compareRanks(candidate, winner) === 0);
        const overridden =
            enforced.length > 0
                ? ordinary
                : ordinary.filter((candidate) => compareRanks(candidate, winner) < 0);

        return {
            setting: this.reference,
            selection,
            value: this.definition.schema.parse(winner.value),
            sources: [...winners.map((candidate) => candidate.source), ...invalid],
            overridden: overridden.map((candidate) => candidate.source),
            enforcement: enforced.length > 0 ? "required" : "ordinary",
        };
    }

    /** Resolve each key of a record value from the highest-ranked candidate that sets it. */
    #mergeKeys(
        selection: SettingSelection,
        ordinary: readonly Candidate[],
        enforced: readonly Candidate[],
        invalid: readonly SettingSource[],
    ): SettingResolution<schema.Infer<Value>> {
        // decide each key from the candidates setting it
        const value: Record<string, JsonValue> = {};
        const keys: NonNullable<SettingResolution["keys"]> = {};
        for (const [key, candidates] of keyCandidates(ordinary, enforced)) {
            const decided = decideKey(key, candidates);
            value[key] = decided.value;
            keys[key] = decided.key;
        }

        // report the default and the candidates deciding a key as sources, the rest as overridden
        const decided = new Set(Object.values(keys).map((key) => key.source));
        const candidates = [...ordinary, ...enforced];
        const isSource = (candidate: Candidate) =>
            candidate.source.kind === "default" || decided.has(candidate.source);

        return {
            setting: this.reference,
            selection,
            value: this.definition.schema.parse(value),
            sources: [
                ...candidates.filter(isSource).map((candidate) => candidate.source),
                ...invalid,
            ],
            overridden: candidates
                .filter((candidate) => !isSource(candidate))
                .map((candidate) => candidate.source),
            enforcement: enforced.length > 0 ? "required" : "ordinary",
            keys,
        };
    }

    /** Require a value the setting's schema accepts. */
    requireValue(value: unknown): schema.Infer<Value> {
        const parsed = this.definition.schema.safeParse(value);
        if (!parsed.success) {
            throw SettingError.of("VALUE_INVALID", "setting value does not match its declaration");
        }

        return parsed.data;
    }

    /** Require a value written in a scope to convert to a valid value of this release. */
    requireWrite(write: SettingWrite, scope: string): void {
        // require a release up to this one and a value it converts to
        const converted = this.#convert(write.value, write.release);
        if (!converted.isConverted) {
            throw SettingError.of(
                "VALUE_INVALID",
                `setting value is at release ${write.release}, but its declaration is at ${this.package.version}`,
            );
        }
        this.requireValue(converted.value);

        // require the placement of the setting's declared scope, or of an enclosing one
        const position = schema.identifier(this.definition.scope).safeParse(scope).success
            ? "declared"
            : "enclosing";
        this.requirePlacement(write, position);
    }

    /** Require a value to have the mode and overrides its scope permits. */
    requirePlacement(
        value: Omit<SettingPlacement, "scope"> & { readonly mode: SettingMode },
        position: "declared" | "enclosing",
    ): void {
        // require set values in the setting's declared scope
        if (position === "declared" && value.mode !== "set") {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                "setting value in its declared scope must be set",
            );
        }
        // require recommendations or requirements from enclosing scopes
        else if (position === "enclosing" && value.mode === "set") {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                "setting value outside its declared scope must be a recommendation or a requirement",
            );
        }

        // refuse an installation together with its space
        if (value.installation !== undefined && value.space !== undefined) {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                "setting value has both an installation and its space",
            );
        }

        // permit the declared overrides in the declared scope, and an installation from enclosing scopes
        const permitted: readonly string[] =
            position === "declared" ? this.definition.overrides : ["installation"];
        const overrides = (
            [
                ["package", value.package],
                ["space", value.space],
                ["installation", value.installation],
            ] as const
        )
            .filter(([, placed]) => placed !== undefined)
            .map(([override]) => override);
        if (overrides.some((override) => !permitted.includes(override))) {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                "setting value uses an unsupported setting override",
            );
        }
    }

    /** Rank a value of this setting that applies to a selection, or none for another value. */
    #candidate(
        value: SettingValue,
        selection: SettingSelection,
        chain: readonly string[],
    ): Candidate | undefined {
        // skip the values of other settings
        if (value.packageId !== this.reference.packageId || value.name !== this.reference.name) {
            return undefined;
        }

        // rank the value, skipping one that does not apply
        const placement = SettingPlacement.of(value);
        const rank = this.#rank(value, placement, selection, chain);
        if (rank === undefined) {
            return undefined;
        }

        // convert the stored value to this release, marking it invalid when the declaration rejects it
        const source = {
            kind: "value" as const,
            id: value.id,
            revision: value.revision,
            ...placement,
        };

        return this.#converted(value, rank, source, value.mode);
    }

    /** Rank a device's value of this setting above every placed one. */
    #deviceCandidate(value: DeviceSettingValue): Candidate | undefined {
        // skip the values of other settings, and refuse one the setting bars from devices
        if (value.packageId !== this.reference.packageId || value.name !== this.reference.name) {
            return undefined;
        } else if (!this.isDeviceStored()) {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                `devices cannot store values of ${DeclarationReference.key(this.reference)}`,
            );
        }

        // convert the device's value to this release, marking it invalid when the declaration rejects it
        const source = { kind: "device" as const, id: value.id, revision: value.revision };

        return this.#converted(value, [DEVICE_TIER, 0], source, "set");
    }

    /** Convert a stored value to this release as a candidate. */
    #converted(
        value: Pick<SettingValue, "value" | "release">,
        rank: Rank,
        source: Extract<SettingSource, { readonly kind: "value" | "device" }>,
        mode: SettingMode,
    ): Candidate {
        const converted = this.#convert(value.value, value.release);
        const parsed = converted.isConverted
            ? this.definition.schema.safeParse(converted.value)
            : undefined;

        return parsed?.success === true
            ? { rank, source, value: parsed.data, mode }
            : { rank, source: { kind: "invalid", source }, value: value.value, mode };
    }

    /** Rank a value of this setting for a selection. */
    #rank(
        value: SettingValue,
        placement: SettingPlacement,
        selection: SettingSelection,
        chain: readonly string[],
    ): Rank | undefined {
        // rank a value set in the selected scope by the overrides matching the selection
        if (value.scope === selection.scope) {
            this.requirePlacement({ ...placement, mode: value.mode }, "declared");
            const weight = setWeight(placement, selection);

            return weight === undefined ? undefined : [SET_TIER, weight];
        }
        // refuse a value set in another scope, such as another user's
        else if (value.mode === "set") {
            throw SettingError.of(
                "PLACEMENT_INVALID",
                "the resolution got a value set in another scope",
            );
        }
        // rank a recommendation or requirement of an enclosing scope by its depth and its installation
        else {
            this.requirePlacement({ ...placement, mode: value.mode }, "enclosing");
            const depth = chain.indexOf(value.scope);
            if (depth === -1) {
                throw SettingError.of(
                    "PLACEMENT_INVALID",
                    "the resolution got a value from outside the selection's scope chain",
                );
            }
            const nearness = (chain.length - depth) * DEPTH_WEIGHT;

            return placement.installation === undefined
                ? [ENCLOSING_TIER, nearness]
                : placement.installation === selection.installation
                  ? [ENCLOSING_TIER, nearness + ENCLOSING_INSTALLATION_WEIGHT]
                  : undefined;
        }
    }

    /** Convert a value of an earlier release to this one, refusing a value of a later release. */
    #convert(
        value: JsonValue,
        release: Version,
    ): { readonly isConverted: true; readonly value: unknown } | { readonly isConverted: false } {
        // refuse a value newer than the declaration
        const current = this.package.version;
        if (Version.compare(release, current) > 0) {
            return { isConverted: false };
        }

        // compute the value through each later release
        const convert = this.definition.convert ?? {};
        const converted = Version.between(convert, release, current).reduce(
            (earlier: JsonValue, [, expression]) =>
                Expression.evaluate(expression, { value: earlier }),
            value,
        );

        return { isConverted: true, value: converted };
    }
}

/** A candidate's precedence: its tier, and its weight within the tier. */
type Rank = readonly [tier: number, weight: number];

/** A value competing for a resolution at a rank. */
interface Candidate {
    /** The precedence among ordinary candidates. */
    readonly rank: Rank;
    /** The value or the default supplying it. */
    readonly source: SettingSource;
    /** The candidate value, converted to this release. */
    readonly value: unknown;
    /** How the value applies. */
    readonly mode: SettingMode;
}

/** Match a value set in the selected scope and weigh its overrides, or none when one differs. */
function setWeight(placement: SettingPlacement, selection: SettingSelection): number | undefined {
    // weigh each override the selection matches
    const matches = [
        { override: placement.package, selected: selection.package, weight: PACKAGE_WEIGHT },
        {
            override: placement.installation,
            selected: selection.installation,
            weight: INSTALLATION_WEIGHT,
        },
        { override: placement.space, selected: selection.space, weight: SPACE_WEIGHT },
    ].filter((match) => match.override !== undefined);

    return matches.every((match) => match.override === match.selected)
        ? matches.reduce((weight, match) => weight + match.weight, 0)
        : undefined;
}

/** The candidates setting one key of a merged setting. */
interface KeyCandidates {
    /** The ordinary candidates, in ascending rank. */
    readonly ordinary: Candidate[];
    /** The requirements. */
    readonly enforced: Candidate[];
}

/** Collect the candidates setting each key of a merged setting's record values. */
function keyCandidates(
    ordinary: readonly Candidate[],
    enforced: readonly Candidate[],
): Map<string, KeyCandidates> {
    const setting = new Map<string, KeyCandidates>();
    for (const [candidates, kind] of [
        [ordinary, "ordinary"],
        [enforced, "enforced"],
    ] as const) {
        for (const candidate of candidates) {
            for (const key of Object.keys(recordOf(candidate.value))) {
                const entry = setting.get(key) ?? { ordinary: [], enforced: [] };
                entry[kind].push(candidate);
                setting.set(key, entry);
            }
        }
    }

    return setting;
}

/** Decide one key: an agreeing requirement, or else the highest-ranked ordinary candidate. */
function decideKey(
    key: string,
    candidates: KeyCandidates,
): { readonly value: JsonValue; readonly key: NonNullable<SettingResolution["keys"]>[string] } {
    // refuse disagreeing requirements
    const entryOf = (candidate: Candidate) => recordOf(candidate.value)[key] ?? null;
    const [requirement] = candidates.enforced;
    if (
        requirement !== undefined &&
        candidates.enforced.some(
            (candidate) => !equalValue(entryOf(candidate), entryOf(requirement)),
        )
    ) {
        throw SettingError.of("VALUES_CONFLICTING", `required values of ${key} conflict`);
    }

    // take the requirement or the highest ranked candidate, refusing a tie of different values
    const winner = requirement ?? nth(candidates.ordinary, -1);
    const tied = candidates.ordinary.filter(
        (candidate) => candidate !== winner && compareRanks(candidate, winner) === 0,
    );
    if (
        requirement === undefined &&
        tied.some((candidate) => !equalValue(entryOf(candidate), entryOf(winner)))
    ) {
        throw SettingError.of(
            "VALUES_CONFLICTING",
            `multiple values of ${key} have the same precedence`,
        );
    }

    return {
        value: entryOf(winner),
        key: {
            source: winner.source,
            overridden: [...candidates.ordinary, ...candidates.enforced]
                .filter((candidate) => candidate !== winner)
                .map((candidate) => candidate.source),
            enforcement: requirement === undefined ? "ordinary" : "required",
        },
    };
}

/** Order candidates by their ranks. */
function compareRanks(left: Candidate, right: Candidate): number {
    return left.rank[0] - right.rank[0] || left.rank[1] - right.rank[1];
}

/** Order candidates by their sources. */
function compareSources(left: Candidate, right: Candidate): number {
    return compareCanonical(left.source, right.source);
}

/** Order JSON values by their canonical form. */
function compareCanonical(left: unknown, right: unknown): number {
    const first = canonicalize(left);
    const second = canonicalize(right);

    return first < second ? -1 : first > second ? 1 : 0;
}

/** Compare JSON values independently of key order. */
function equalValue(left: unknown, right: unknown): boolean {
    return canonicalize(left) === canonicalize(right);
}

/** Read a merged setting's value as its record of keys. */
function recordOf(value: unknown): JsonObject {
    return schema.jsonObject().parse(value);
}

/** A value's setting at a placement, its release absent until a stack stamps it. */
export type SettingDeclared = DeclarationReference &
    SettingPlacement &
    Partial<Pick<SettingWrite, "release">>;

/** A view a setting's declaring build is read in: a call's or an apply's. */
export interface SettingView {
    /** Read a table's rows in the view. */
    readonly read: <Read extends Table>(table: Read) => RelationalQueryBuilder<Read>;
}

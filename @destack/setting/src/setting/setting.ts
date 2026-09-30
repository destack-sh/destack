import { PackageId, type Package } from "@destack/package";
import { defineSchema, identifier, schema, Version } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { Condition, Expression } from "@destack/db/query";
import type { JsonValue } from "@destack/db";
import type { SettingDefinition } from "../declare/setting.ts";
import type { SettingRow } from "../object/setting.ts";
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

/** An enclosing recommendation one scope nearer follows a farther one, whatever its installation. */
const DEPTH_WEIGHT = 2;
/** A recommendation for the selected installation follows one for every installation of its scope. */
const ENCLOSING_INSTALLATION_WEIGHT = 1;

/** A set value for the consuming package follows one applying to every package. */
const PACKAGE_WEIGHT = 1;
/** A set value for the space follows the package override alone. */
const SPACE_WEIGHT = 2;
/** A set value for the installation follows one for its space. */
const INSTALLATION_WEIGHT = 4;
/** A device-specific value follows every device-independent one. */
const DEVICE_WEIGHT = 8;

/** A package-local setting name, kept across releases. */
export const SettingName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/),
);

/** The identity of a setting across package renames and releases. */
export const SettingReference = Object.assign(
    defineSchema(
        schema.object({
            /** The package declaring the setting. */
            packageId: PackageId,
            /** The package-local name. */
            name: SettingName,
        }),
    ),
    {
        /** Key a setting by its package and name. */
        key(reference: SettingReference): string {
            return `${reference.packageId}/${reference.name}`;
        },
    },
);
/** The identity of a setting. */
export type SettingReference = schema.Infer<typeof SettingReference>;

/** A value written in a scope, as a write or a stack's declaration carries it. */
export type SettingWrite = Omit<SettingPlacement, "scope"> & {
    /** How the value applies. */
    readonly mode: SettingMode;
    /** The value. */
    readonly value: unknown;
    /** The release of the setting's package the value was written against. */
    readonly release: Version;
};

/** A typed setting declaration. */
export class Setting<Value extends schema.Schema = schema.Schema> {
    /** The declaring package. */
    readonly package: Package;
    /** The name, schema, default, scopes and conversions. */
    readonly definition: SettingDefinition<Value>;

    /** Hold a checked declaration. */
    constructor(owner: Package, definition: SettingDefinition<Value>) {
        this.package = owner;
        this.definition = definition;
    }

    /** The identity its values refer to. */
    get reference(): SettingReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the setting as its reference. */
    toJSON(): SettingReference {
        return this.reference;
    }

    /** Match the values a resolution reads: those set in the selected scope, and the recommendations and requirements above it. */
    condition(selection: SettingSelection): Condition {
        const policy = Condition.ne("mode", "set");

        return Condition.all(
            Condition.eq("packageId", this.reference.packageId),
            Condition.eq("name", this.reference.name),
            selection.scope === null
                ? policy
                : Condition.any(Condition.eq("scope", selection.scope), policy),
        );
    }

    /**
     * Resolve the effective value for a selection from the values placed along a scope chain, nearest scope first.
     *
     * A stored value the declaration no longer accepts is skipped and listed as an invalid source.
     */
    resolve(
        selection: SettingSelection,
        values: readonly SettingRow[],
        chain: readonly string[],
    ): SettingResolution<schema.Infer<Value>> {
        // require a selection of the setting's scope, or an anonymous one of a user setting
        const isSelectable =
            selection.scope === null
                ? this.definition.scope === "user"
                : identifier(this.definition.scope).safeParse(selection.scope).success;
        if (!isSelectable) {
            throw new SettingError(
                "INVALID_PLACEMENT",
                "setting scope does not match the selected scope",
            );
        }

        // collect the default and each applicable value of this setting, apart from invalid ones
        const ordinary: Candidate[] = [
            {
                rank: [DEFAULT_TIER, 0],
                source: { kind: "default", package: this.package },
                value: this.definition.default,
                mode: "set",
            },
        ];
        const required: Candidate[] = [];
        const invalid: SettingSource[] = [];
        for (const value of values) {
            const candidate = this.#candidate(value, selection, chain);
            if (candidate === undefined) {
                continue;
            } else if (candidate.source.kind === "invalid") {
                invalid.push(candidate.source);
            } else {
                (candidate.mode === "require" ? required : ordinary).push(candidate);
            }
        }

        // order candidates independently of arrival order
        ordinary.sort((left, right) => compareRanks(left, right) || compareSources(left, right));
        required.sort(compareSources);
        invalid.sort((left, right) => compareCanonical(left, right));

        // reject equally ranked candidates other than agreeing recommendations
        for (let index = 1; index < ordinary.length; index++) {
            const previous = ordinary[index - 1]!;
            const current = ordinary[index]!;
            const isShared =
                previous.mode === "recommend" &&
                current.mode === "recommend" &&
                equalValue(previous.value, current.value);
            if (compareRanks(previous, current) === 0 && !isShared) {
                throw new SettingError(
                    "CONFLICT",
                    "multiple setting values have the same precedence",
                );
            }
        }

        // reject disagreeing requirements
        if (required.some((candidate) => !equalValue(candidate.value, required[0]!.value))) {
            throw new SettingError("CONFLICT", "required setting values disagree");
        }

        // take the requirements, or else the highest ranked candidates
        const winner = required[0] ?? ordinary[ordinary.length - 1]!;
        const winners =
            required.length > 0
                ? required
                : ordinary.filter((candidate) => compareRanks(candidate, winner) === 0);
        const overridden =
            required.length > 0
                ? ordinary
                : ordinary.filter((candidate) => compareRanks(candidate, winner) < 0);

        return {
            setting: this.reference,
            selection,
            value: winner.value as schema.Infer<Value>,
            sources: [...winners.map((candidate) => candidate.source), ...invalid],
            overridden: overridden.map((candidate) => candidate.source),
            enforcement: required.length > 0 ? "required" : "ordinary",
        };
    }

    /** Require a value the setting's schema accepts. */
    requireValue(value: unknown): schema.Infer<Value> {
        const parsed = this.definition.schema.safeParse(value);
        if (!parsed.success) {
            throw new SettingError("INVALID_VALUE", "setting value does not match its declaration");
        }

        return parsed.data;
    }

    /** Require a value written in a scope to convert to a valid value of this release, at a permitted placement. */
    requireWrite(write: SettingWrite, scope: string): void {
        // require a release up to this one and a value it converts to
        const converted = this.#convert(write.value, write.release);
        if (!converted.isConverted) {
            throw new SettingError(
                "INVALID_VALUE",
                `setting value is at release ${write.release}, its declaration at ${this.package.version}`,
            );
        }
        this.requireValue(converted.value);

        // require the placement of the setting's own scope, or of an enclosing one
        const position = identifier(this.definition.scope).safeParse(scope).success
            ? "own"
            : "enclosing";
        this.requirePlacement(write, position);
    }

    /** Require a value to carry the mode and overrides its scope permits. */
    requirePlacement(
        value: Omit<SettingPlacement, "scope"> & { readonly mode: SettingMode },
        position: "own" | "enclosing",
    ): void {
        // require set values in the setting's own scope
        if (position === "own" && value.mode !== "set") {
            throw new SettingError(
                "INVALID_PLACEMENT",
                "setting value in its declared scope must be set",
            );
        }
        // require recommendations or requirements from enclosing scopes
        else if (position === "enclosing" && value.mode === "set") {
            throw new SettingError(
                "INVALID_PLACEMENT",
                "setting value outside its declared scope must recommend or require",
            );
        }

        // refuse an installation together with its space
        if (value.installation !== undefined && value.space !== undefined) {
            throw new SettingError(
                "INVALID_PLACEMENT",
                "setting value carries both an installation and its space",
            );
        }

        // permit the declared overrides in the own scope, and an installation from enclosing scopes
        const permitted: readonly string[] =
            position === "own" ? this.definition.overrides : ["installation"];
        const overrides = (["package", "space", "installation", "device"] as const).filter(
            (override) => value[override] !== undefined,
        );
        if (overrides.some((override) => !permitted.includes(override))) {
            throw new SettingError(
                "INVALID_PLACEMENT",
                "setting value uses an unsupported setting override",
            );
        }
    }

    /** Rank a value of this setting that applies to a selection, or none for another value. */
    #candidate(
        value: SettingRow,
        selection: SettingSelection,
        chain: readonly string[],
    ): Candidate | undefined {
        // skip the values of other settings
        if (value.packageId !== this.reference.packageId || value.name !== this.reference.name) {
            return undefined;
        }

        // rank a value set in the selected scope by the overrides matching the selection
        const placement = SettingPlacement.of(value);
        let rank: Rank | undefined;
        if (value.scope === selection.scope) {
            this.requirePlacement({ ...placement, mode: value.mode }, "own");
            const weight = setWeight(placement, selection);
            rank = weight === undefined ? undefined : [SET_TIER, weight];
        }
        // refuse a value set in another scope, such as another user's
        else if (value.mode === "set") {
            throw new SettingError(
                "INVALID_PLACEMENT",
                "a value set in another scope reached the resolution",
            );
        }
        // rank a recommendation or requirement of an enclosing scope by its depth, then its installation
        else {
            this.requirePlacement({ ...placement, mode: value.mode }, "enclosing");
            const depth = chain.indexOf(value.scope);
            if (depth === -1) {
                throw new SettingError(
                    "INVALID_PLACEMENT",
                    "a value from outside the selection's scope chain reached the resolution",
                );
            }
            const nearness = (chain.length - depth) * DEPTH_WEIGHT;
            rank =
                placement.installation === undefined
                    ? [ENCLOSING_TIER, nearness]
                    : placement.installation === selection.installation
                      ? [ENCLOSING_TIER, nearness + ENCLOSING_INSTALLATION_WEIGHT]
                      : undefined;
        }

        // skip values that do not apply
        if (rank === undefined) {
            return undefined;
        }

        // convert the stored value to this release, and mark it invalid when the declaration rejects it
        const placed = { id: value.id, revision: value.revision, ...placement };
        const converted = this.#convert(value.value, value.release);
        const parsed = converted.isConverted
            ? this.definition.schema.safeParse(converted.value)
            : undefined;
        if (parsed?.success !== true) {
            return {
                rank,
                source: { kind: "invalid", ...placed },
                value: value.value,
                mode: value.mode,
            };
        }

        return {
            rank,
            source: { kind: "value", ...placed },
            value: parsed.data,
            mode: value.mode,
        };
    }

    /** Convert a value of an earlier release to this one, refusing a value of a later release. */
    #convert(
        value: unknown,
        release: Version,
    ): { readonly isConverted: true; readonly value: unknown } | { readonly isConverted: false } {
        // refuse a value newer than the declaration
        const current = this.package.version;
        if (Version.compare(release, current) > 0) {
            return { isConverted: false };
        }

        // compute the value through each later release
        const convert = this.definition.convert ?? {};
        const converted = Version.between(Object.keys(convert), release, current).reduce(
            (earlier: JsonValue, target) =>
                Expression.evaluate(convert[target]!, { value: earlier }),
            value as JsonValue,
        );

        return { isConverted: true, value: converted };
    }
}

/** A candidate's precedence: its tier, then its weight within the tier. */
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
        { override: placement.device, selected: selection.device, weight: DEVICE_WEIGHT },
    ].filter((match) => match.override !== undefined);

    return matches.every((match) => match.override === match.selected)
        ? matches.reduce((weight, match) => weight + match.weight, 0)
        : undefined;
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

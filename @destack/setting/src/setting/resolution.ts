import { sameSubject } from "@destack/access";
import { Package } from "@destack/package";
import { defineSchema, identifier, schema } from "@destack/schema";
import { Setting, SettingReference } from "./setting.ts";
import { SettingSelection, SettingTarget } from "./target.ts";
import type { SettingAssignment } from "./assignment.ts";
import type { SettingPolicy } from "./policy.ts";
import { SettingError } from "../error/error.ts";

/** Explicit assignments follow every recommendation. */
const ASSIGNMENT_PRIORITY = 4;
/** Device-specific assignments follow the corresponding device-independent targets. */
const DEVICE_PRIORITY = 8;

/** The declaration or exact persisted revision supplying a candidate value. */
export const SettingSource = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The consuming release's declaration default. */
            kind: schema.literal("default"),
            /** The declaration version supplying this default and value schema. */
            package: Package,
        }),
        schema.object({
            /** An explicit assignment revision. */
            kind: schema.literal("assignment"),
            /** The contributing assignment. */
            id: identifier("setting-assignment"),
            /** The contributing revision. */
            revision: schema.uuid(),
            /** Exact target configured by this assignment. */
            target: SettingTarget,
        }),
        schema.object({
            /** A recommended or required policy revision. */
            kind: schema.literal("policy"),
            /** The contributing policy. */
            id: identifier("setting-policy"),
            /** The contributing revision. */
            revision: schema.uuid(),
        }),
    ]),
);
/** The source of a resolved candidate. */
export type SettingSource = schema.Infer<typeof SettingSource>;

/** Authority-filtered explanation of a resolved value. */
export const SettingResolution = defineSchema(
    schema.object({
        /** The stable declaration identity. */
        setting: SettingReference,
        /** The verified caller and receiving runtime selection. */
        target: SettingSelection,
        /** The effective value, checked against the consuming declaration. */
        value: schema.json(),
        /** All equal required policies, or the single winning ordinary source. */
        sources: schema.array(SettingSource).min(1),
        /** Other applicable value sources in ascending precedence order. */
        overridden: schema.array(SettingSource),
        /** Whether mandatory policy fixes the effective value. */
        enforcement: schema.enum(["ordinary", "required"]),
        /** Remote policy expiry in UTC milliseconds, or null for current local authority. */
        validUntil: schema.number().int().nonnegative().nullable(),
    }),
);
/** A resolved value with its authorized explanation. */
export type SettingResolution<Value = schema.Infer<ReturnType<typeof schema.json>>> = Omit<
    schema.Infer<typeof SettingResolution>,
    "value"
> & {
    /** The inferred setting value. */
    value: Value;
};

/** Complete authorized inputs supplied by the host within one policy freshness interval. */
export interface SettingSnapshot {
    /** The represented user or receiving shared runtime. */
    readonly target: SettingSelection;
    /** Assignments the caller and consuming package may read. */
    readonly assignments: readonly SettingAssignment[];
    /** Applicable policies already restricted by administrative scope and delegation. */
    readonly policies: readonly SettingPolicy[];
    /** Exclusive remote policy expiry; null denotes current locally authoritative policy. */
    readonly validUntil: number | null;
}

/** Resolve one declaration from complete host-authorized inputs. */
export function resolveSetting<Value extends schema.Schema>(
    setting: Setting<Value>,
    snapshot: SettingSnapshot,
    now: number,
): SettingResolution<schema.Infer<Value>> {
    // refuse stale policy inputs, including stale absence of required policy
    if (
        snapshot.validUntil !== null &&
        (!Number.isFinite(snapshot.validUntil) || now >= snapshot.validUntil)
    ) {
        throw new SettingError("STALE_POLICY", "setting policy selection has expired");
    }
    if (snapshot.target.kind !== setting.declaration.scope) {
        throw new SettingError(
            "INVALID_TARGET",
            "setting scope does not match the selected target",
        );
    }

    // order ordinary sources independently of transport arrival order
    const ordinary: Candidate[] = [
        {
            rank: 0,
            source: { kind: "default", package: setting.declaration.package },
            value: setting.declaration.default,
        },
    ];
    const required: Candidate[] = [];
    for (const policy of snapshot.policies) {
        if (!sameSetting(policy.setting, setting.reference)) {
            continue;
        }
        const candidate = {
            rank: policy.installationId ? 2 : 1,
            source: { kind: "policy" as const, id: policy.id, revision: policy.revision },
            value: policy.value,
        };
        if (policy.mode === "required") {
            required.push(candidate);
        } else {
            ordinary.push(candidate);
        }
    }

    // include only explicit values whose target matches this invocation
    for (const assignment of snapshot.assignments) {
        if (!sameSetting(assignment.setting, setting.reference)) {
            continue;
        }
        const rank = assignmentRank(assignment.target, snapshot.target, setting);
        if (rank !== undefined) {
            ordinary.push({
                rank: ASSIGNMENT_PRIORITY + rank,
                source: {
                    kind: "assignment",
                    id: assignment.id,
                    revision: assignment.revision,
                    target: assignment.target,
                },
                value: assignment.value,
            });
        }
    }

    // require valid values even when another source currently overrides them
    const candidates = [...ordinary, ...required];
    for (const candidate of candidates) {
        if (!setting.declaration.schema.safeParse(candidate.value).success) {
            throw new SettingError(
                "INVALID_VALUE",
                "stored value is incompatible with the setting declaration",
            );
        }
    }

    // reject disagreeing equally specific recommendations and duplicate assignments
    ordinary.sort(
        (left, right) =>
            left.rank - right.rank ||
            JSON.stringify(left.source).localeCompare(JSON.stringify(right.source)),
    );
    required.sort((left, right) =>
        JSON.stringify(left.source).localeCompare(JSON.stringify(right.source)),
    );
    for (let index = 1; index < ordinary.length; index++) {
        const previous = ordinary[index - 1];
        const current = ordinary[index];
        if (previous.rank === current.rank) {
            if (
                previous.source.kind !== "policy" ||
                current.source.kind !== "policy" ||
                !equalValue(previous.value, current.value)
            ) {
                throw new SettingError(
                    "CONFLICT",
                    "multiple setting values have the same precedence",
                );
            }
        }
    }
    if (required.some((candidate) => !equalValue(candidate.value, required[0].value))) {
        throw new SettingError("CONFLICT", "required setting policies disagree");
    }

    // retain every agreeing mandatory source in the explanation
    const winner = required[0] ?? ordinary[ordinary.length - 1];
    const winners = required.length
        ? required
        : ordinary.filter((candidate) => candidate.rank === winner.rank);
    const sources = winners.map((candidate) => candidate.source);
    const overridden = (
        required.length ? ordinary : ordinary.filter((candidate) => candidate.rank < winner.rank)
    ).map((candidate) => candidate.source);

    return {
        setting: setting.reference,
        target: snapshot.target,
        value: setting.declaration.schema.parse(winner.value),
        sources,
        overridden,
        enforcement: required.length ? "required" : "ordinary",
        validUntil: snapshot.validUntil,
    };
}

/** A contributing source at a fixed precedence. */
interface Candidate {
    /** Ordinary precedence, ignored for required policies. */
    rank: number;
    /** The persisted revision or declaration default. */
    source: SettingSource;
    /** A value checked before returning a resolution. */
    value: unknown;
}

/** Read an exact target's edit revision from the same snapshot as its displayed value. */
export function assignmentRevision(
    resolution: SettingResolution<unknown>,
    target: SettingTarget,
): string | null {
    const selected = SettingTarget.parse(target);
    const source = [...resolution.sources, ...resolution.overridden].find(
        (source) => source.kind === "assignment" && equalValue(source.target, selected),
    );

    return source?.kind === "assignment" ? source.revision : null;
}

/** Compare declaration identity without using mutable package names. */
function sameSetting(left: SettingReference, right: SettingReference): boolean {
    return left.packageId === right.packageId && left.name === right.name;
}

/** Match an assignment and rank its permitted refinements. */
function assignmentRank(
    candidate: SettingTarget,
    selected: SettingSelection,
    setting: Setting,
): number | undefined {
    // separate personal, shared and host configuration
    if (candidate.kind !== selected.kind) {
        return undefined;
    }
    if (candidate.kind === "host" && selected.kind === "host") {
        return candidate.hostId === selected.hostId ? 0 : undefined;
    }
    if (
        candidate.kind === "user" &&
        selected.kind === "user" &&
        (selected.user === null || !sameSubject(candidate.user, selected.user))
    ) {
        return undefined;
    }

    // reject unsupported refinements even when this invocation has no matching override
    setting.assertTarget(candidate);

    // compare the containing space before any installation refinement
    const location = "location" in candidate ? candidate.location : undefined;
    const selectedLocation = "location" in selected ? selected.location : undefined;
    let rank = 0;
    if (candidate.kind === "user" && candidate.packageId) {
        if (selected.kind !== "user" || candidate.packageId !== selected.packageId) {
            return undefined;
        }
        rank = 1;
    }
    if (location) {
        if (
            location.spaceId !== selectedLocation?.spaceId ||
            (location.installationId &&
                location.installationId !== selectedLocation?.installationId)
        ) {
            return undefined;
        }
        rank += location.installationId ? 4 : 2;
    }

    // place device-specific equivalents above all ordinary personal targets
    if (candidate.kind === "user" && candidate.deviceId) {
        if (!("deviceId" in selected) || candidate.deviceId !== selected.deviceId) {
            return undefined;
        }
        rank += DEVICE_PRIORITY;
    }

    return rank;
}

/** Compare JSON values independently of object key insertion order. */
function equalValue(left: unknown, right: unknown): boolean {
    if (left === right) {
        return true;
    }
    if (!left || !right || typeof left !== "object" || typeof right !== "object") {
        return false;
    }
    if (Array.isArray(left) || Array.isArray(right)) {
        return (
            Array.isArray(left) &&
            Array.isArray(right) &&
            left.length === right.length &&
            left.every((value, index) => equalValue(value, right[index]))
        );
    }

    // compare member sets without allocating sorted copies
    const members = Object.entries(left);

    return (
        members.length === Object.keys(right).length &&
        members.every(
            ([key, value]) =>
                Object.hasOwn(right, key) &&
                equalValue(value, (right as Record<string, unknown>)[key]),
        )
    );
}

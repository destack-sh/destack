import { DependencyName } from "@destack/package/package";
import { ResourceName } from "@destack/resource";
import { defineSchema, schema } from "@destack/schema";

/** The decision applied to a matching package. */
export const PackageDecision = defineSchema(schema.enum(["allow", "deny"]));
/** A package policy decision. */
export type PackageDecision = schema.Infer<typeof PackageDecision>;

/** A selection of actual packages, independent of dependency import aliases. */
export const PackageSelector = defineSchema(schema.object({
    /** The package format, including source packages authored for Destack. */
    kind: schema.enum(["npm", "destack"]),
    /** The canonical registry URL; omission matches any registry of this format. */
    registry: schema.string().regex(/^https?:\/\/[^\s?#@]+\/$/).optional(),
    /** An exact package name; omission matches every name. */
    name: DependencyName.optional(),
    /** An npm version range; omission matches every version. */
    version: schema.string().min(1).optional(),
    /** Exact content integrity; omission permits any content matching the other fields. */
    integrity: schema.string().min(1).optional(),
}));
/** A package selection used by a policy rule. */
export type PackageSelector = schema.Infer<typeof PackageSelector>;

/** A named allow or deny rule. */
export const PackageRule = defineSchema(schema.object({
    /** All specified selector fields must match. */
    package: PackageSelector,
    /** Deny takes precedence over allow within this policy. */
    decision: PackageDecision,
}));
/** A package policy rule. */
export type PackageRule = schema.Infer<typeof PackageRule>;

/** Decisions for a complete resolved package graph. */
export const PackageRules = defineSchema(schema.object({
    /** The decision when no rule matches. */
    default: PackageDecision,
    /** Rules keyed by stable names used in diagnostics and audit records. */
    rules: schema.record(ResourceName, PackageRule),
}));
/** Package rules evaluated without rule ordering. */
export type PackageRules = schema.Infer<typeof PackageRules>;

/** Package admission for a space and its installations. */
export const PackagePolicyDefinition = defineSchema(schema.object({
    /** Rules for the root package and every resolved dependency. */
    admission: PackageRules,
}));
/** A source-authored or interactively managed package policy. */
export type PackagePolicyDefinition = schema.Infer<typeof PackagePolicyDefinition>;

import { Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { FeatureMetadata, FeatureReset } from "./feature.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The fields of every feature description beside its kind. */
const description = FeatureMetadata.extend({
    /** The declaring package. */
    package: Package,
});

/** A feature declaration as manifests describe it. */
export const FeatureDescription = defineSchema(
    schema.discriminatedUnion("kind", [
        description.extend({
            /** Access alone. */
            kind: schema.literal("boolean"),
        }),
        description.extend({
            /** A fixed value each grant sets. */
            kind: schema.literal("static"),
            /** The JSON Schema of the value a grant sets. */
            value: schema.record(schema.string(), schema.json()),
        }),
        description.extend({
            /** Usage of meters up to the limit a grant sets. */
            kind: schema.literal("metered"),
            /** The meters of one unit whose events count together as usage. */
            meters: schema.array(CatalogReference).min(1),
            /** When the usage starts again. */
            reset: FeatureReset,
        }),
    ]),
);
/** A feature declaration as manifests describe it. */
export type FeatureDescription = schema.Infer<typeof FeatureDescription>;

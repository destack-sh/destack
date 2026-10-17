import { Package } from "@destack/package";
import { defineSchema, type schema } from "@destack/schema";
import { MeterMetadata } from "./meter.ts";

/** A meter declaration as manifests describe it. */
export const MeterDescription = defineSchema(
    MeterMetadata.extend({
        /** The declaring package. */
        package: Package,
    }),
);
/** A meter declaration as manifests describe it. */
export type MeterDescription = schema.Infer<typeof MeterDescription>;

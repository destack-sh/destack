import { defineSchema, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";

/** Associate a generated file with its ECMA-426 source map in this build. */
export const SourceMapReference = defineSchema(schema.object({
    /** The generated JavaScript, CSS, or other mapped file. */
    generated: PackagePath,
    /** The source map file, including maps with indexed sections. */
    map: PackagePath,
}));
/** A generated file and its source map. */
export type SourceMapReference = schema.Infer<typeof SourceMapReference>;

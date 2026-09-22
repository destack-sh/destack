import { defineSchema, schema } from "@destack/schema";

import { PackagePath } from "../file/file.ts";

/** A position in an original or generated source file. */
export const SourceLocation = defineSchema(
    schema.object({
        /** The file path relative to the package or build root. */
        file: PackagePath,
        /** The zero-based line. */
        line: schema.number().int().min(0),
        /** The zero-based UTF-16 column. */
        column: schema.number().int().min(0),
    }),
);
/** A file position measured in lines and UTF-16 columns. */
export type SourceLocation = schema.Infer<typeof SourceLocation>;

/** A source range measured in zero-based UTF-16 offsets. */
export const SourceRange = defineSchema(
    schema.object({
        /** The source file relative to the package root. */
        file: PackagePath,
        /** The inclusive start offset. */
        start: schema.number().int().min(0),
        /** The exclusive end offset. */
        end: schema.number().int().min(0),
    }),
);
/** A source range measured in zero-based UTF-16 offsets. */
export type SourceRange = schema.Infer<typeof SourceRange>;

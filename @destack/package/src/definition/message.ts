import { schema } from "@destack/schema";
import { PackageId } from "./package.ts";

/** A package's localizable text: its package, identifier, MessageFormat 2 source and values. */
export const Message = schema.object({
    /** The package that wrote it, whose catalogs translate it. */
    package: PackageId,
    /** The stable identifier catalogs translate it under. */
    id: schema.string().min(1),
    /** The source text in the package's language, as MessageFormat 2. */
    source: schema.string(),
    /** The values of its placeholders and selectors, by variable name. */
    values: schema.jsonObject(),
});
/** A package's localizable text. */
export type Message = schema.Infer<typeof Message>;

import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, DependencyName } from "./package.ts";

/** A function a package exports, as export and name, such as `./inspect#describeDatabase`. */
export const FunctionReference = defineSchema(
    schema.string().regex(/^\.(?:\/[a-z][a-z0-9-]*)*#[A-Za-z][A-Za-z0-9]*$(?![\s\S])/u),
);
/** A function a package exports, as export and name. */
export type FunctionReference = schema.Infer<typeof FunctionReference>;

/** A declaration constructor as its package declares it in destack.json. */
export const DeclarationConstructor = defineSchema(
    schema.object({
        /** The descriptions the build writes for each declaration, absent when it writes none. */
        describes: schema
            .array(
                schema.object({
                    /** The description kind. */
                    kind: DeclarationName,
                    /** The dependency declaring the kind, this package when absent. */
                    package: DependencyName.exactOptional(),
                    /** The describing function. */
                    function: FunctionReference,
                    /** The function comparing two releases' descriptions into changes. */
                    compare: FunctionReference.exactOptional(),
                    /** The function listing a description's terms with their definitions. */
                    vocabulary: FunctionReference.exactOptional(),
                }),
            )
            .min(1)
            .exactOptional(),
        /** The parameter position where the transform passes the calling module, if any. */
        module: schema.number().int().nonnegative().exactOptional(),
    }),
);
/** A declaration constructor as its package declares it in destack.json. */
export type DeclarationConstructor = schema.Infer<typeof DeclarationConstructor>;

/** The declaration constructors a package declares, by name. */
export const DeclarationConstructorMap = defineSchema(
    schema.record(schema.string().regex(/^define[A-Z][A-Za-z0-9]*$/u), DeclarationConstructor),
);
/** The declaration constructors a package declares, by name. */
export type DeclarationConstructorMap = schema.Infer<typeof DeclarationConstructorMap>;

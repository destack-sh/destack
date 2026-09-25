import { defineSchema, schema } from "@destack/schema";
import { DeclarationName } from "./package.ts";

/** A declaration constructor as its package declares it in destack.json. */
export const DeclarationConstructor = defineSchema(
    schema.object({
        /** How the build describes the declarations, absent for constructors it does not inspect. */
        inspect: schema
            .object({
                /** The manifest description kind. */
                kind: DeclarationName,
                /** The describing function, as a package export and export name such as `./inspect#describeDatabase`. */
                describe: schema
                    .string()
                    .regex(/^\.(?:\/[a-z][a-z0-9-]*)*#[A-Za-z][A-Za-z0-9]*$(?![\s\S])/),
            })
            .optional(),
        /** The parameter position where the transform passes the calling module, if any. */
        module: schema.number().int().nonnegative().optional(),
    }),
);
/** A declaration constructor as its package declares it in destack.json. */
export type DeclarationConstructor = schema.Infer<typeof DeclarationConstructor>;

/** The declaration constructors a package declares, by name. */
export const DeclarationConstructorMap = defineSchema(
    schema.record(schema.string().regex(/^define[A-Z][A-Za-z0-9]*$/), DeclarationConstructor),
);
/** The declaration constructors a package declares, by name. */
export type DeclarationConstructorMap = schema.Infer<typeof DeclarationConstructorMap>;

import { schema } from "@destack/schema";
import { DeclarationName, DependencyName } from "./package.ts";

/** A function a package exports, as export and name, such as `./inspect#describeDatabase`. */
export const FunctionReference = schema
    .string()
    .regex(/^\.(?:\/[a-z][a-z0-9-]*)*#[A-Za-z][A-Za-z0-9]*$/u);
/** A function a package exports, as export and name. */
export type FunctionReference = schema.Infer<typeof FunctionReference>;

/** The definition of a kind of declaration the package owns: its describing functions. */
export const DeclarationKindDefinition = schema.object({
    /** The description kind. */
    kind: DeclarationName,
    /** The describing function. */
    function: FunctionReference,
    /** The function comparing two releases' descriptions into changes. */
    compare: FunctionReference.exactOptional(),
    /** The function listing a declaration's terms with their definitions. */
    vocabulary: FunctionReference.exactOptional(),
    /** The function listing the symbols a declaration derives with their relationships. */
    symbols: FunctionReference.exactOptional(),
});
/** The definition of a kind of declaration the package owns. */
export type DeclarationKindDefinition = schema.Infer<typeof DeclarationKindDefinition>;

/** A dependency's kind each declaration makes, as a value the kind describes. */
const KindValueRead = schema.object({
    /** The description kind. */
    kind: DeclarationName,
    /** The dependency declaring the kind. */
    package: DependencyName,
    /** The function reading the kind's value from a declaration. */
    value: FunctionReference,
});

/** A dependency's kind each declaration makes, described by this package's own functions. */
const KindValueDescribed = DeclarationKindDefinition.extend({
    /** The dependency declaring the kind. */
    package: DependencyName,
});

/** A dependency's kind each declaration makes: as a value, or described by this package. */
export const DeclarationKindValue = schema.union([KindValueRead, KindValueDescribed]);
/** A dependency's kind each declaration also makes. */
export type DeclarationKindValue = schema.Infer<typeof DeclarationKindValue>;

/** A declaration constructor the build describes, as its package declares it in destack.json. */
export const DeclarationConstructor = schema.object({
    /** The kinds the build describes each declaration as. */
    describes: schema.array(schema.union([DeclarationKindDefinition, DeclarationKindValue])).min(1),
});
/** A declaration constructor the build describes. */
export type DeclarationConstructor = schema.Infer<typeof DeclarationConstructor>;

/** The declaration constructors a package's build describes, by name. */
export const DeclarationConstructorMap = schema.record(
    schema.string().regex(/^define[A-Z][A-Za-z0-9]*$/u),
    DeclarationConstructor,
);
/** The declaration constructors a package's build describes, by name. */
export type DeclarationConstructorMap = schema.Infer<typeof DeclarationConstructorMap>;

/** A function, template tag or declaration constructor that receives the calling module. */
export const Stamp = schema.object({
    /** The parameter position where the transform passes the calling module. */
    module: schema.number().int().nonnegative(),
});
/** A function the module transform passes the calling module to. */
export type Stamp = schema.Infer<typeof Stamp>;

/** The functions receiving the calling module, by export path such as `Message.context`. */
export const StampMap = schema.record(
    schema.string().regex(/^[A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)*$/u),
    Stamp,
);
/** The functions a package's callers receive the calling module in, by export path. */
export type StampMap = schema.Infer<typeof StampMap>;

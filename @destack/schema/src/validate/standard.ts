import * as z from "zod";

/** A validator of any library implementing the Standard Schema protocol, version 1, as standardschema.dev declares it. */
export interface StandardSchema<Input = unknown, Output = Input> {
    /** The protocol's properties. */
    readonly "~standard": StandardProperties<Input, Output>;
}

/** The properties a Standard Schema validator carries under `~standard`. */
export interface StandardProperties<Input = unknown, Output = Input> {
    /** The protocol version. */
    readonly version: 1;
    /** The library implementing the validator. */
    readonly vendor: string;
    /** Validate a value into the output or the issues found. */
    readonly validate: (value: unknown) => StandardResult<Output> | Promise<StandardResult<Output>>;
    /** The input and output types, for inference only. */
    readonly types?: { readonly input: Input; readonly output: Output } | undefined;
}

/** A Standard Schema validation's result: the output, or the issues found. */
export type StandardResult<Output> =
    | { readonly value: Output; readonly issues?: undefined }
    | { readonly issues: readonly StandardIssue[] };

/** One issue a Standard Schema validation found. */
export interface StandardIssue {
    /** The issue's message. */
    readonly message: string;
    /** Where the issue is, as property keys or path segments. */
    readonly path?: readonly (PropertyKey | { readonly key: PropertyKey })[] | undefined;
}

/** Validate a value as a Standard Schema validator: its protocol version, vendor and validate function. */
export function standard(): z.ZodType<StandardSchema> {
    return z.custom<StandardSchema>((value) => {
        const properties =
            typeof value === "object" && value !== null && "~standard" in value
                ? value["~standard"]
                : undefined;

        return (
            typeof properties === "object" &&
            properties !== null &&
            "version" in properties &&
            properties.version === 1 &&
            "vendor" in properties &&
            typeof properties.vendor === "string" &&
            "validate" in properties &&
            typeof properties.validate === "function"
        );
    }, "expected a Standard Schema validator");
}

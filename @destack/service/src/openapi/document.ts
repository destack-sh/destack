import { schema, toJsonSchema } from "@destack/schema";
import {
    type JSONSchema,
    type ConditionalSchemaConverter,
    OpenAPIGenerator,
    type OpenAPIGeneratorGenerateOptions,
} from "@orpc/openapi";
import type { ServiceRouter } from "../service/index.ts";

/** Convert portable Destack schemas for HTTP decoding and OpenAPI documents. */
export const schemaConverter: ConditionalSchemaConverter = {
    condition: (validator) => validator instanceof schema.Schema,
    convert: (validator) => {
        if (!(validator instanceof schema.Schema)) {
            throw new TypeError("expected a Destack schema");
        }

        return [true, toJsonSchema(validator) as JSONSchema];
    },
};

/** Configure document metadata, servers, security schemes, and shared schemas. */
export type DocumentOptions = OpenAPIGeneratorGenerateOptions;

/** Generate OpenAPI from the service's routes and portable Destack schemas. */
export function createDocument(definition: ServiceRouter, options: DocumentOptions) {
    // use the same schema restrictions as other Destack packages
    const generator = new OpenAPIGenerator({
        schemaConverters: [schemaConverter],
    });

    return generator.generate(definition, options);
}

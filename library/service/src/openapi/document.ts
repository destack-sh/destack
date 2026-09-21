import { schema, toJsonSchema } from "@destack/schema";
import {
    type JSONSchema,
    OpenAPIGenerator,
    type OpenAPIGeneratorGenerateOptions,
} from "@orpc/openapi";
import type { Service } from "../service/index.ts";

/** Configure document metadata, servers, security schemes, and shared schemas. */
export type DocumentOptions = OpenAPIGeneratorGenerateOptions;

/** Generate OpenAPI from the service's routes and portable Destack schemas. */
export function createDocument(definition: Service, options: DocumentOptions) {
    // use the same schema restrictions as other Destack packages
    const generator = new OpenAPIGenerator({
        schemaConverters: [
            {
                condition: (validator) => validator instanceof schema.Schema,
                convert: (validator) => {
                    if (!(validator instanceof schema.Schema)) {
                        throw new TypeError("expected a Destack schema");
                    }

                    // both libraries describe JSON Schema Draft 2020-12 with distinct TypeScript types
                    return [true, toJsonSchema(validator) as JSONSchema];
                },
            },
        ],
    });

    return generator.generate(definition, options);
}

import { identifier as defineIdentifier, type schema } from "@destack/schema";
import { toJsonSchema } from "@destack/schema/inspect";
import { customType } from "drizzle-orm/sqlite-core";

/** Define a TEXT column containing a typed UUIDv7 identifier. */
export function identifier<const Prefix extends string>(name: string, prefix: Prefix) {
    // retain the identifier validator for queries and schema inspection
    const validator = defineIdentifier(prefix);
    const description = toJsonSchema(validator);
    const column = customType<{
        data: schema.Infer<typeof validator>;
        driverData: string;
    }>({
        dataType: () => "text",
        toDriver: (value) => validator.parse(value),
        fromDriver: (value) => validator.parse(value),
    })(name);

    return Object.assign(column, { schema: validator, jsonSchema: description });
}

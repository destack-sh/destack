import * as schema from "@destack/schema/validate";
import { toJsonSchema } from "@destack/schema/inspect";
import { customType } from "drizzle-orm/sqlite-core";

/** Define a SQLite TEXT column with validated JSON values. */
export function json<T extends schema.Schema>(name: string, validator: T) {
    // check portability once when the column is declared
    const description = toJsonSchema(validator);

    // preserve the schema description in the custom column configuration
    const column = customType<{
        data: schema.Output<T>;
        driverData: string;
        config: { jsonSchema: typeof description };
        configRequired: true;
    }>({
        dataType: () => "text",
        toDriver(value) {
            // reject non JSON values before serialization can discard them
            schema.json().parse(value);
            const parsed = validator.parse(value);

            return JSON.stringify(parsed);
        },
        fromDriver(value) {
            return validator.parse(JSON.parse(value));
        },
    })(name, { jsonSchema: description });

    return Object.assign(column, { schema: validator, jsonSchema: description });
}

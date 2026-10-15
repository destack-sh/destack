import { type JsonValue, schema } from "@destack/schema";

/** An input's object of fields, each parsing to a JSON value. */
export type InputObject = schema.Object<{ readonly [Field in string]: schema.Schema<JsonValue> }>;

/** A method's complete input: an object of fields, or one per sum type variant by one field. */
export type InputSchema = InputObject | schema.DiscriminatedUnion<readonly InputObject[]>;

/** What every method input answers. */
export const InputSchema = {
    /** Read every field an input takes, each variant's included. */
    fields(input: InputSchema): InputObject {
        return input instanceof schema.Object
            ? input
            : schema.object(
                  Object.fromEntries(
                      input.options.flatMap((option) => Object.entries(option.shape)),
                  ),
              );
    },

    /** Change the object of an input, or each variant's. */
    map(input: InputSchema, change: (object: InputObject) => InputObject): InputSchema {
        return input instanceof schema.Object
            ? change(input)
            : InputSchema.union(input.def.discriminator, input.options.map(change));
    },

    /** Join one object per variant into a union told apart by a field, refusing no variant. */
    union(field: string, variants: readonly InputObject[]): InputSchema {
        const [first, ...others] = variants;
        if (first === undefined) {
            throw new TypeError(`a union by ${field} has no variant`);
        }

        return schema.discriminatedUnion(field, [first, ...others]);
    },
};

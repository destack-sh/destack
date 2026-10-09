import { schema } from "@destack/schema";

/** A method's complete input: an object of fields, or one object per variant of a sum type, told apart by one field. */
export type InputSchema =
    | schema.JsonObject
    | schema.DiscriminatedUnion<readonly schema.JsonObject[]>;

/** What every method input answers. */
export const InputSchema = {
    /** Read every field an input takes, each variant's included. */
    fields(input: InputSchema): schema.JsonObject {
        return input instanceof schema.Object
            ? input
            : schema.object(
                  Object.fromEntries(
                      input.options.flatMap((option) => Object.entries(option.shape)),
                  ),
              );
    },

    /** Change the object of an input, or each variant's. */
    map(input: InputSchema, change: (object: schema.JsonObject) => schema.JsonObject): InputSchema {
        return input instanceof schema.Object
            ? change(input)
            : InputSchema.union(input.def.discriminator, input.options.map(change));
    },

    /** Join one object per variant into a union told apart by a field, refusing no variant. */
    union(field: string, variants: readonly schema.JsonObject[]): InputSchema {
        const [first, ...others] = variants;
        if (first === undefined) {
            throw new TypeError(`a union by ${field} has no variant`);
        }

        return schema.discriminatedUnion(field, [first, ...others]);
    },
};

import { NumberFormat } from "@destack/locale";
import { schema, toJsonSchema } from "@destack/schema";

/** What a JSON Schema says of a number's limits. */
const LIMITED = schema.looseObject({
    type: schema.unknown().exactOptional(),
    minimum: schema.number().exactOptional(),
    maximum: schema.number().exactOptional(),
    multipleOf: schema.number().positive().exactOptional(),
});

/** How far binary fractions leave a multiple from a whole count of steps. */
const STEP_NOISE = 1e-9;

/** A schema as the property of an object. */
const PROPERTY = schema.looseObject({ properties: schema.object({ value: schema.unknown() }) });

/** The numbers an input takes, and how people read them. */
const numberInput = schema.object({
    /** The lowest number. */
    minimum: schema.number().exactOptional(),
    /** The highest number. */
    maximum: schema.number().exactOptional(),
    /** The step between numbers, one for whole ones. */
    step: schema.number().positive().exactOptional(),
    /** How people read them. */
    format: NumberFormat.exactOptional(),
});
/** The numbers an input takes, and how people read them. */
export type NumberInput = schema.Infer<typeof numberInput>;
/** The numbers an input takes, and how people read them. */
export const NumberInput = Object.assign(numberInput, {
    /** Read the numbers a schema takes. */
    of(validator: schema.Schema, format?: NumberFormat): NumberInput {
        // describe it as a property, where a schema may have a default
        const described = PROPERTY.parse(toJsonSchema(schema.object({ value: validator })));

        return NumberInput.ofJson(described.properties.value, format);
    },

    /** Read the numbers a JSON Schema takes, leaving out the limits every whole number has. */
    ofJson(description: unknown, format?: NumberFormat): NumberInput {
        const { type, minimum, maximum, multipleOf } = LIMITED.parse(description);
        const step = multipleOf ?? (type === "integer" ? 1 : undefined);

        return {
            ...(minimum === undefined || minimum === Number.MIN_SAFE_INTEGER ? {} : { minimum }),
            ...(maximum === undefined || maximum === Number.MAX_SAFE_INTEGER ? {} : { maximum }),
            ...(step === undefined ? {} : { step }),
            ...(format === undefined ? {} : { format }),
        };
    },

    /** Report whether an input takes a number. */
    takes({ minimum, maximum, step }: NumberInput, value: number): boolean {
        // count the steps from zero, as a schema's multiples do
        const steps = step === undefined ? 0 : value / step;

        return (
            Number.isFinite(value) &&
            Math.abs(steps - Math.round(steps)) < STEP_NOISE &&
            value >= (minimum ?? Number.NEGATIVE_INFINITY) &&
            value <= (maximum ?? Number.POSITIVE_INFINITY)
        );
    },
});

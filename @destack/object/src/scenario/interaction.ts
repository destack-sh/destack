import type { Interaction } from "@destack/package/declare";
import { type JsonObject, type JsonValue, present, schema } from "@destack/schema";

/** A reference to an object a call bound: `$name` the latest, `$name.3` one of many. */
const OBJECT_REFERENCE = /^\$([a-z][a-zA-Z0-9-]*)(?:\.(\d+))?$/u;

/** A reference to a person's principal by name, as `@alice`. */
const PERSON_REFERENCE = /^@([a-z][a-zA-Z0-9-]*)$/u;

/** A call of an object's method as a person, its input referring to bound objects and to people. */
export const ObjectStep = Object.assign(
    schema.object({
        /** The type and method, such as `notebook.create`. */
        call: schema.string().regex(/^[a-z][a-zA-Z0-9]*\.[a-z][a-zA-Z0-9]*$/u),
        /** The person calling, by name, such as `alice`. */
        as: schema.string().min(1),
        /** The input, `$name` a bound object's identifier and `@name` a person's principal. */
        input: schema.jsonObject(),
        /** The name the called object's identifier binds to, in call order. */
        bind: schema.string().min(1).exactOptional(),
    }),
    {
        /** Read the type and method a call names. */
        target(call: { readonly call: string }): {
            readonly type: string;
            readonly method: string;
        } {
            const [type, method] = call.call.split(".");

            return {
                type: present(type, `the type ${call.call} names`),
                method: present(method, `the method ${call.call} names`),
            };
        },

        /** Resolve a call's input: bound objects to identifiers, people to principals. */
        input(
            input: JsonObject,
            bound: ReadonlyMap<string, readonly string[]>,
            principal: (person: string) => JsonValue,
        ): JsonObject {
            return schema.jsonObject().parse(resolve(input, bound, principal));
        },
    },
);
/** A call of an object's method as a person. */
export type ObjectStep = schema.Infer<typeof ObjectStep>;

/** An observation of an object world: the objects of a type it holds. */
export const ObjectObservation = schema.object({
    /** Count the objects of a type, such as `notebook`. */
    count: schema.string().min(1),
});
/** An observation of an object world. */
export type ObjectObservation = schema.Infer<typeof ObjectObservation>;

/** The interaction of object worlds: people calling the methods of the packages a store serves. */
export const objectInteraction: Interaction<
    ObjectStep,
    ObjectObservation,
    Record<string, never>
> = Object.freeze({
    name: "object",
    step: ObjectStep,
    observation: ObjectObservation,
    globals: schema.object({}),
});

/** Resolve a value's references to bound objects and people, refusing a reference to none. */
function resolve(
    value: unknown,
    bound: ReadonlyMap<string, readonly string[]>,
    principal: (person: string) => JsonValue,
): unknown {
    // replace a bound object's identifier, the latest or the numbered one
    if (typeof value === "string") {
        const object = OBJECT_REFERENCE.exec(value);
        const person = PERSON_REFERENCE.exec(value)?.[1];
        if (object !== null) {
            const [, name = "", index] = object;
            const identifiers = bound.get(name) ?? [];
            const identifier =
                index === undefined ? identifiers.at(-1) : identifiers[Number(index)];
            if (identifier === undefined) {
                throw new TypeError(`'${value}' refers to no object bound as '${name}'`);
            }

            return identifier;
        }
        // replace a person by their principal
        else if (person !== undefined) {
            return principal(person);
        }
        // leave any other string
        else {
            return value;
        }
    }
    // resolve each item and field
    else if (Array.isArray(value)) {
        return value.map((each) => resolve(each, bound, principal));
    } else if (typeof value === "object" && value !== null) {
        return Object.fromEntries(
            Object.entries(value).map(([key, each]) => [key, resolve(each, bound, principal)]),
        );
    }
    // leave any other value
    else {
        return value;
    }
}

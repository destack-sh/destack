import { type JsonValue, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { ObjectInput, type PlaceVariable } from "./variable.ts";

/** The values a part of a page gives to the actions in it, by variable. */
export class Place {
    /** The values, by the key of their variable. */
    readonly #values: ReadonlyMap<string, JsonValue>;

    /** Create a place without values, or with those by the keys of their variables. */
    constructor(values: ReadonlyMap<string, JsonValue> = new Map()) {
        this.#values = values;
    }

    /** Create a place with a current object, whatever its type. */
    static current(reference: ObjectReference): Place {
        return new Place(new Map([[ObjectInput.keyOf("current", reference), reference]]));
    }

    /** Create a place with selected objects, each with those of its type. */
    static selected(references: readonly ObjectReference[]): Place {
        const byType = Map.groupBy(references, (reference) =>
            ObjectInput.keyOf("selected", reference),
        );

        return new Place(byType);
    }

    /** Give a variable's value too. */
    with<Value extends JsonValue>(variable: PlaceVariable<Value>, value: Value): Place {
        return new Place(new Map([...this.#values, [variable.key, value]]));
    }

    /** Read the value a key has here, absent for none. */
    get(key: string): JsonValue | undefined {
        return this.#values.get(key);
    }

    /** List the objects selected at some places, the nearest of each type. */
    static selection(places: readonly Place[]): ObjectReference[] {
        // take each type's nearest
        const nearest = new Map<string, JsonValue>();
        for (const place of places.toReversed()) {
            for (const [key, value] of place.#values) {
                if (key.startsWith(ObjectInput.prefixOf("selected"))) {
                    nearest.set(key, value);
                }
            }
        }

        return [...nearest.values()].flatMap((value) => schema.array(ObjectReference).parse(value));
    }

    /** Read the nearest value some keys have, at the first place with one of them. */
    static nearest(places: readonly Place[], keys: readonly string[]): JsonValue | undefined {
        return places
            .flatMap((place) => keys.map((key) => place.get(key)))
            .find((value) => value !== undefined);
    }
}

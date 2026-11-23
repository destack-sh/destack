import type { JsonValue } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { ObjectInput, type PlaceVariable } from "./variable.ts";

/** The values a part of a page gives to the actions in it, by variable. */
export class Place {
    /** The values, by the key of their variable. */
    readonly #values: ReadonlyMap<string, JsonValue>;

    /** Create a place without values, or with those by the keys of their variables. */
    constructor(values: ReadonlyMap<string, JsonValue> = new Map()) {
        this.#values = values;
    }

    /** Create a place showing an object, whatever its type. */
    static showing(reference: ObjectReference): Place {
        return new Place(new Map([[ObjectInput.keyOf("current", reference), reference]]));
    }

    /** Give a variable's value too. */
    with<Value extends JsonValue>(variable: PlaceVariable<Value>, value: Value): Place {
        return new Place(new Map([...this.#values, [variable.key, value]]));
    }

    /** Read the value a key has here, absent for none. */
    get(key: string): JsonValue | undefined {
        return this.#values.get(key);
    }

    /** Read the nearest value some keys have, at the first place with one of them. */
    static nearest(places: readonly Place[], keys: readonly string[]): JsonValue | undefined {
        return places
            .flatMap((place) => keys.map((key) => place.get(key)))
            .find((value) => value !== undefined);
    }
}

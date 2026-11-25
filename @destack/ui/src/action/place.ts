import { Place } from "@destack/object";
import type { ObjectReference } from "@destack/sync";
import { provide } from "@destack/view";
import { items } from "./action.ts";

/** The items a list gives to the actions in it, by their keys, and the objects they stand for. */
export interface ItemPlace {
    /** List the items a person is on at an element, none apart from them. */
    readonly targets: (element: Element | null) => readonly string[];
    /** Read the object an item stands for, absent for items of none. */
    readonly reference?: ((key: string) => ObjectReference | undefined) | undefined;
}

/** Give the items a person is on to the actions in the element this is the `ref` of. */
export function provideItems(place: ItemPlace): (element: Element) => void {
    return provide((element) => placeOf(place.targets(element), place.reference));
}

/** Build the place of some items, with the objects they stand for selected. */
export function placeOf(keys: readonly string[], reference?: ItemPlace["reference"]): Place {
    // give nothing where the person is on none
    if (keys.length === 0) {
        return new Place();
    }
    const references = keys.flatMap((key) => reference?.(key) ?? []);

    return Place.selected(references).with(items, keys);
}

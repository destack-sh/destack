import { fromBase64, fromHex, toBase64, toHex } from "./bytes.ts";
import { escape } from "./regexp.ts";
import {
    difference,
    intersection,
    isDisjointFrom,
    isSubsetOf,
    isSupersetOf,
    symmetricDifference,
    union,
} from "./set.ts";
import { parse } from "./url.ts";

/** The browsers that ship `Uint8Array`'s hex and base64 methods natively. */
const BYTES = "Safari 18.2, Chrome 140, Firefox 133";

/** The browsers that ship `Set`'s set-algebra methods natively. */
const SETS = "Safari 17, Chrome 122, Firefox 127";

/** A web-standard member newer than the Destack browser baseline, and its spec implementation. */
export interface Shim {
    /** The built-in the member belongs to, such as `Uint8Array.prototype`. */
    readonly owner: object;
    /** The member's name on its owner. */
    readonly name: string;
    /** The member, implemented as its specification defines it. */
    readonly implementation: (this: never, ...values: never[]) => unknown;
    /** The browsers that ship the member natively, from which the shim can retire. */
    readonly native: string;
}

/** The members the baseline installs where a browser lacks them, by the built-in they extend. */
export const BASELINE: readonly Shim[] = [
    // bytes as hex and base64
    { owner: Uint8Array.prototype, name: "toHex", implementation: toHex, native: BYTES },
    { owner: Uint8Array.prototype, name: "toBase64", implementation: toBase64, native: BYTES },
    { owner: Uint8Array, name: "fromHex", implementation: fromHex, native: BYTES },
    { owner: Uint8Array, name: "fromBase64", implementation: fromBase64, native: BYTES },

    // set algebra
    { owner: Set.prototype, name: "union", implementation: union, native: SETS },
    { owner: Set.prototype, name: "intersection", implementation: intersection, native: SETS },
    { owner: Set.prototype, name: "difference", implementation: difference, native: SETS },
    {
        owner: Set.prototype,
        name: "symmetricDifference",
        implementation: symmetricDifference,
        native: SETS,
    },
    { owner: Set.prototype, name: "isSubsetOf", implementation: isSubsetOf, native: SETS },
    { owner: Set.prototype, name: "isSupersetOf", implementation: isSupersetOf, native: SETS },
    { owner: Set.prototype, name: "isDisjointFrom", implementation: isDisjointFrom, native: SETS },

    // text and addresses
    {
        owner: RegExp,
        name: "escape",
        implementation: escape,
        native: "Safari 18.2, Chrome 136, Firefox 134",
    },
    { owner: URL, name: "parse", implementation: parse, native: "Safari 18, Chrome 126, Firefox 126" },
];

/** Install each baseline member a browser lacks, as the platform defines its built-ins. */
export function installBaseline(): void {
    for (const shim of BASELINE) {
        if (!(shim.name in shim.owner)) {
            Object.defineProperty(shim.owner, shim.name, {
                value: shim.implementation,
                writable: true,
                configurable: true,
            });
        }
    }
}

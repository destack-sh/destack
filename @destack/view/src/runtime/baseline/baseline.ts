import { fromBase64, fromHex, Uint8ArrayMethods } from "./bytes.ts";
import { escape } from "./regexp.ts";
import { SetMethods } from "./set.ts";
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
    /** The members implementing it as its specification defines it, by name. */
    readonly members: object;
    /** The browsers that ship the member natively, from which the shim can retire. */
    readonly native: string;
}

/** The members each built-in gains where a browser lacks them, and the browsers that ship them. */
const GROUPS: readonly Omit<Shim, "name">[] = [
    // bytes as hex and base64
    { owner: Uint8Array.prototype, members: Uint8ArrayMethods, native: BYTES },
    { owner: Uint8Array, members: { fromHex, fromBase64 }, native: BYTES },

    // set algebra
    { owner: Set.prototype, members: SetMethods, native: SETS },

    // text and addresses
    { owner: RegExp, members: { escape }, native: "Safari 18.2, Chrome 136, Firefox 134" },
    { owner: URL, members: { parse }, native: "Safari 18, Chrome 126, Firefox 126" },
];

/** The members the baseline installs where a browser lacks them, by the built-in they extend. */
export const BASELINE: readonly Shim[] = GROUPS.flatMap((group) =>
    Object.keys(group.members).map((name) => ({ ...group, name })),
);

/** Install each baseline member a browser lacks, as the platform defines its built-ins. */
export function installBaseline(): void {
    for (const shim of BASELINE) {
        if (!(shim.name in shim.owner)) {
            Object.defineProperty(shim.owner, shim.name, {
                value: Reflect.get(shim.members, shim.name),
                writable: true,
                configurable: true,
            });
        }
    }
}

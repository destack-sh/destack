import { BYTES_SHIMS } from "./bytes.ts";
import { REGEXP_SHIMS } from "./regexp.ts";
import { SET_SHIMS } from "./set.ts";
import type { NativeVersions, Shim } from "./shim.ts";
import { URL_SHIMS } from "./url.ts";

export type * from "./shim.ts";

/** The browsers shipping `Symbol.dispose` and `Symbol.asyncDispose`, per browser-compat-data. */
const DISPOSAL: NativeVersions = { chrome: 134, edge: 134, firefox: 141 };

/** The members the baseline installs where a browser lacks them. */
export const BASELINE: readonly Shim[] = [
    ...BYTES_SHIMS,
    ...SET_SHIMS,
    ...REGEXP_SHIMS,
    ...URL_SHIMS,

    // disposal, as the registered symbols the compiler's disposal helpers fall back to
    {
        feature: "javascript.builtins.Symbol.dispose",
        owner: Symbol,
        name: "dispose",
        implementation: Symbol.for("Symbol.dispose"),
        native: DISPOSAL,
    },
    {
        feature: "javascript.builtins.Symbol.asyncDispose",
        owner: Symbol,
        name: "asyncDispose",
        implementation: Symbol.for("Symbol.asyncDispose"),
        native: DISPOSAL,
    },
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

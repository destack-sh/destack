/** A browser engine Destack builds for, as esbuild names its targets. */
export type Browser = "chrome" | "edge" | "firefox" | "safari";

/** The first version of each browser shipping a member, a missing browser shipping none yet. */
export type NativeVersions = Readonly<Partial<Record<Browser, number>>>;

/** A web-standard member a target browser may lack, as browser-compat-data names it. */
export interface Shim {
    /** The member's browser-compat-data path, such as `javascript.builtins.Set.union`. */
    readonly feature: string;
    /** The built-in the member belongs to, such as `Set.prototype`. */
    readonly owner: object;
    /** The member's name on its owner. */
    readonly name: string;
    /** The member as its specification defines it. */
    readonly implementation: unknown;
    /** The first version of each browser shipping it, per browser-compat-data. */
    readonly native: NativeVersions;
}

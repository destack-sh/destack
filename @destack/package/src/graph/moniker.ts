import { nth, schema } from "@destack/schema";

/** The pattern of a moniker: package and module path, then optional symbol, member and kind. */
const MONIKER_PATTERN =
    /^(package-[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}|npm:(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*)\/[^#\x00-\x1f\x7f]+(?:#[^\x00-\x1f\x7f]+)?$/u;

/** The schema of a moniker. */
const monikerSchema = schema.string().regex(MONIKER_PATTERN);

/** A stable moniker of a module, symbol or declaration across builds. */
export type Moniker = schema.Infer<typeof monikerSchema>;

/** A stable name of a module, symbol or declaration across builds. */
export const Moniker = Object.assign(monikerSchema, { of, packageId, module });

/** The names a moniker of a module, symbol or member joins. */
export interface MonikerName {
    /** The package identifier, or `npm:<name>` for a package without one. */
    readonly packageId: string;
    /** The package-relative module path. */
    readonly module: string;
    /** The qualified symbol name, absent for the module itself. */
    readonly name?: string;
    /** The member of the symbol. */
    readonly member?: string;
}

/** Join a moniker from its names. */
function of(name: MonikerName): Moniker {
    // name the module followed by the symbol and its member
    const symbol = name.name === undefined ? "" : `#${name.name}`;
    const member = name.member === undefined ? "" : `.${name.member}`;

    return monikerSchema.parse(`${name.packageId}/${name.module}${symbol}${member}`);
}

/** Read the package identifier a moniker starts with, or `npm:<name>` for a package without one. */
function packageId(moniker: Moniker): string {
    const [, owner] = MONIKER_PATTERN.exec(moniker) ?? [];
    if (owner === undefined) {
        throw new TypeError(`not a moniker: ${moniker}`);
    }

    return owner;
}

/** Read a moniker's package-relative module path, past any symbol, member or kind. */
function module(moniker: Moniker): string {
    const rest = moniker.slice(packageId(moniker).length + 1);

    return nth(nth(rest.split("#"), 0).split(":"), 0);
}

import { plugin } from "bun";
import { present } from "@destack/schema";

/** The link of each builtin specifier, process-wide as Bun's module registry is. */
const LINKS = new Map<string, Promise<void>>();

/** Link the specifiers of the builtins to this process's own modules. */
export async function linkBuiltins(
    specifiers: readonly string[],
    builtins: ReadonlySet<string>,
): Promise<void> {
    // link each builtin specifier not linked yet, then wait for every link the caller needs, in flight ones included
    const needed = [...new Set(specifiers)].filter((specifier) => {
        const name = specifier.split("/").slice(0, 2).join("/");

        return builtins.has(name);
    });
    for (const specifier of needed.filter((each) => !LINKS.has(each))) {
        LINKS.set(specifier, link(specifier));
    }
    await Promise.all(
        needed.map((specifier) => present(LINKS.get(specifier), `the link of ${specifier}`)),
    );
}

/** Register a specifier as a virtual module answering this process's own instance. */
async function link(specifier: string): Promise<void> {
    // read the module this process resolves the specifier to, before registering, as a registered specifier answers itself
    const path = Bun.resolveSync(specifier, import.meta.dirname);
    const exports: unknown = await import(path);
    if (typeof exports !== "object" || exports === null) {
        throw new TypeError(`the builtin module ${specifier} at ${path} exports nothing`);
    }
    const namespace = { ...exports };

    // register it as a virtual module answering that instance
    plugin({
        name: `destack-builtin ${specifier}`,
        setup(build) {
            build.module(specifier, () => ({ exports: namespace, loader: "object" }));
        },
    });
}

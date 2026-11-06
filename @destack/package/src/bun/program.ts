import type { ModuleLoader } from "@destack/environment";
import { present } from "@destack/schema";
import { DependencyName } from "../definition/package.ts";
import { SHARED_MODULES } from "./shared.ts";

/** Load a shared module as the program itself does. */
export type ModuleLoad = () => Promise<object>;

/** A module's exports by name. */
type Namespace = Readonly<Record<string, unknown>>;

/** A running program: the modules it shares with the package code it loads, one instance each. */
export class Program {
    /** The shared specifiers known before loading: a compiled table's, none from files. */
    readonly specifiers: readonly string[];
    /** Read how the program loads a module, absent for one it does not share. */
    readonly #load: (specifier: string) => ModuleLoad | undefined;
    /** The program's module loader, which defines its shared modules for the code it loads. */
    readonly #loader: ModuleLoader;
    /** The program's own instance of each module read so far, by specifier. */
    readonly #namespaces = new Map<string, Promise<Namespace>>();
    /** The definition of each linked specifier as a module answering the program's instance. */
    readonly #linked = new Map<string, Promise<void>>();

    /** Run a program sharing the modules a function says how it loads, through its loader. */
    constructor(
        load: (specifier: string) => ModuleLoad | undefined,
        loader: ModuleLoader,
        specifiers: readonly string[] = [],
    ) {
        this.#load = load;
        this.#loader = loader;
        this.specifiers = specifiers;
    }

    /** Read a compiled program, sharing the table of modules its build generated. */
    static of(table: Readonly<Record<string, ModuleLoad>>, loader: ModuleLoader): Program {
        return new Program((specifier) => table[specifier], loader, Object.keys(table));
    }

    /** Read a program run from files, sharing every module of some packages from their directories. */
    static resolving(packages: ReadonlyMap<string, string>, loader: ModuleLoader): Program {
        // load a specifier of a shared package from the file its dependent resolves it to
        const load = (specifier: string): ModuleLoad | undefined => {
            const directory = packages.get(DependencyName.of(specifier));

            return directory === undefined
                ? undefined
                : async (): Promise<object> => {
                      const loaded: unknown = await import(loader.resolve(specifier, directory));

                      return readNamespace(loaded);
                  };
        };

        return new Program(load, loader);
    }

    /** Read the running program: its compiled table, else its packages resolved from files. */
    static async current(
        packages: () => Promise<ReadonlyMap<string, string>>,
        loader: ModuleLoader,
    ): Promise<Program> {
        // take the table a compiled program's build generated, else resolve from the program's files
        return SHARED_MODULES === undefined
            ? Program.resolving(await packages(), loader)
            : Program.of(SHARED_MODULES, loader);
    }

    /** Whether the program shares a module. */
    shares(specifier: string): boolean {
        return this.#load(specifier) !== undefined;
    }

    /** Read the program's own instance of a shared module, loading it once. */
    async module(specifier: string): Promise<Namespace> {
        // refuse a module the program does not share
        const load = this.#load(specifier);
        if (load === undefined) {
            throw new TypeError(`the program shares no module ${specifier}`);
        }

        // load it once
        const namespace = this.#namespaces.get(specifier) ?? load().then(readNamespace);
        this.#namespaces.set(specifier, namespace);

        return await namespace;
    }

    /** Answer the shared ones among some specifiers with the program's own instances. */
    async link(specifiers: Iterable<string>): Promise<void> {
        // link each shared specifier once, then wait for every link the code needs, in flight ones included
        const needed = [...new Set(specifiers)].filter((specifier) => this.shares(specifier));
        for (const specifier of needed.filter((each) => !this.#linked.has(each))) {
            this.#linked.set(specifier, this.#define(specifier));
        }
        await Promise.all(
            needed.map((specifier) => present(this.#linked.get(specifier), specifier)),
        );
    }

    /** Define a specifier as a module answering the program's own instance. */
    async #define(specifier: string): Promise<void> {
        // read the instance before defining, as a defined specifier answers itself
        const exports = await this.module(specifier);
        this.#loader.define(specifier, { ...exports });
    }
}

/** Read a loaded module as its exports by name, keeping the module itself as the one instance. */
function readNamespace(loaded: unknown): Namespace {
    // refuse anything but a module's object, without copying it
    if (!isNamespace(loaded)) {
        throw new TypeError("a shared module loaded as no object of exports");
    }

    return loaded;
}

/** Report whether a loaded value is an object of exports. */
function isNamespace(loaded: unknown): loaded is Namespace {
    return typeof loaded === "object" && loaded !== null;
}

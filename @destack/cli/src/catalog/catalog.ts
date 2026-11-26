import { Account } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import machinePackage from "@destack/machine/package";
import updatePackage from "@destack/update/package";
import settingPackage from "@destack/setting/package";
import objectPackage from "@destack/object/package";
import type { ObjectAddress } from "@destack/view/declare";
import type { SpaceAddress } from "@destack/account/address";
import {
    ACTION_KIND,
    type PackageImplementationDescription,
    ObjectType,
    ObjectTypePool,
    type Package,
    PackageMount,
    Casing,
    ObjectTypeEntry,
} from "@destack/object";
import registryPackage from "@destack/registry/package";
import { ObjectStoreDescription, ObjectConnection } from "@destack/object/client";
import { nth, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import stackPackage from "@destack/stack/package";
import { Setting, SETTING_KIND } from "@destack/setting";

import type { CommandContext } from "../command/context.ts";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";
import type { CatalogType } from "./type.ts";
import type { Command } from "../command/command.ts";

/** The separator of a qualified noun: the installation's name, then the noun. */
const QUALIFIER = ":";

/** The platform packages built into Destack, whose types the command line knows. */
const BUILT_IN_PACKAGES: readonly Package[] = [
    machinePackage,
    updatePackage,
    accountPackage,
    settingPackage,
    stackPackage,
    objectPackage,
];

/** The package each store read over the network describes itself at: one of its scope's own. */
const DESCRIBING = {
    device: machinePackage.id,
    machine: machinePackage.id,
    universe: accountPackage.id,
    space: stackPackage.id,
} as const;

/** The stores the command line reads nouns from, outermost first. */
const STORES = ["device", "machine", "universe", "registry", "space"] as const;

/** The levels each store keeps objects at, the one it reads first for a type of any scope. */
const OWN_LEVELS = {
    device: ["device"],
    machine: ["machine"],
    universe: ["user", "account", Scope.universe.id],
    registry: ["space"],
    space: ["space"],
} as const satisfies Readonly<Record<(typeof STORES)[number], readonly string[]>>;

/** A store the command line reads nouns from: device, machine, universe, registry or space. */
export type Store = (typeof STORES)[number];

/** An object type the catalog lists: its entry, the package serving it and the store. */
export class CatalogEntry {
    /** The package listing the type, as its store lists it. */
    readonly package: PackageImplementationDescription;
    /** The store running the installation. */
    readonly store: Store;
    /** The type's entry. */
    readonly entry: ObjectTypeEntry;
    /** The store's description, declaring the type once the noun opens. */
    readonly #description: ObjectStoreDescription;

    /** Keep a type's entry of the package listing it, in a store, with the store's description. */
    constructor(
        implementation: PackageImplementationDescription,
        store: Store,
        entry: ObjectTypeEntry,
        description: ObjectStoreDescription,
    ) {
        // keep the entry, where it is listed, and the description declaring its type
        this.package = implementation;
        this.store = store;
        this.entry = entry;
        this.#description = description;
    }

    /** Read the singular word. */
    get name(): string {
        return Casing.kebab(this.entry.type);
    }

    /** Read the plural word, which lists. */
    get plural(): string {
        return Casing.kebab(this.entry.plural);
    }

    /** Read the qualifier: the installation's name, else its package's name without the scope. */
    get qualifier(): string {
        const name = this.package.installation?.name;
        const { packageName } = this.entry;

        return name ?? packageName.slice(packageName.lastIndexOf("/") + 1);
    }

    /** List the words of the noun: its plural and its singular. */
    get forms(): readonly string[] {
        return [this.plural, this.name];
    }

    /** Report whether a word is the noun's plural, which lists. */
    lists(word: string): boolean {
        return word === this.plural || word === this.qualify(this.plural);
    }

    /** Qualify a word of the noun by its qualifier. */
    qualify(word: string): string {
        return `${this.qualifier}${QUALIFIER}${word}`;
    }

    /** Whether the store keeps the objects at a level of its own, or they live in any scope. */
    get isOwn(): boolean {
        const own: readonly string[] = OWN_LEVELS[this.store];
        const { levels } = this.entry;

        return levels.length === 0 || levels.some((level) => own.includes(level));
    }

    /** Open the entry: its type, declared on first use, with the actions calling its methods. */
    async open(context: CommandContext): Promise<CatalogType> {
        // declare the type and read its actions, loading the commands' module on first use
        const type = await this.#description.type(this.entry);
        const actions = await this.#description.actions(type);
        const opened = await context.module(() => import("./type.ts"));

        return new opened.CatalogType(this, type, actions);
    }

    /** Open a client of the type's objects in the scope the terminal uses for them. */
    async client(context: CommandContext): Promise<ObjectConnection> {
        return CatalogEntry.connect(context, this.store, this.package, await this.#locate(context));
    }

    /** Locate the scope the type's objects live in for the person: the store's for several. */
    async #locate(context: CommandContext): Promise<string> {
        // read the store's own level the type lives in, else its one level, any for a type of any scope
        const { levels } = this.entry;
        const own = OWN_LEVELS[this.store];
        const owned = own.find((candidate) => levels.includes(candidate));
        const [level = nth(own, 0), ...others] = owned === undefined ? levels : [owned];
        const refusal = `${this.plural} live in ${levels.join(" or ")} scopes, which the terminal does not resolve`;

        // read the universe
        if (level === Scope.universe.id) {
            return Scope.universe.id;
        }
        // refuse a type of several scopes, which the terminal cannot choose among
        else if (others.length > 0) {
            throw CliError.of("USAGE_INVALID", refusal);
        }
        // read the home serving the account's registry
        else if (level === "space" && this.store === "registry") {
            return Account.home(await context.account());
        }
        // read the space the command line acts in
        else if (level === "space") {
            return (await context.locate()).placement.space;
        }
        // read the account the space belongs to
        else if (level === "account") {
            return context.account();
        }
        // read the person signed in
        else if (level === "user") {
            return (await context.launcher()).user.userId;
        }
        // read this computer's device
        else if (level === "device") {
            return (await context.native()).id;
        }
        // read this machine
        else if (level === "machine") {
            const { machine } = await (await context.native()).loopback.read();

            return machine;
        }
        // refuse other scopes
        else {
            throw CliError.of("USAGE_INVALID", refusal);
        }
    }

    /** Connect to a package implementation's store, closed once the command line ran. */
    static async connect(
        context: CommandContext,
        store: Store,
        implementation: Pick<PackageImplementationDescription, "packageId" | "installation">,
        scope?: string,
    ): Promise<ObjectConnection> {
        // close the connection once the command line ran
        const connection = await CatalogEntry.#open(context, store, implementation, scope);
        context.defer(() => connection.close());

        return connection;
    }

    /** Open a connection to a package implementation's store, as the person or the owner. */
    static async #open(
        context: CommandContext,
        store: Store,
        implementation: Pick<PackageImplementationDescription, "packageId" | "installation">,
        scope?: string,
    ): Promise<ObjectConnection> {
        // connect to this computer's device, and its machine's partition by the partition header
        const { packageId } = implementation;
        const name = implementation.installation?.name;
        const native = await context.native();
        const path = PackageMount.path(packageId);
        if (store === "device") {
            return native.loopback.connection(path, scope);
        } else if (store === "machine") {
            const { machine } = await native.loopback.read();

            return native.loopback.connection(path, scope, machine);
        }

        // connect to the universe at the person's issuer, stepping up as a call asks
        const launcher = await context.launcher();
        const { elevate } = context.consent();
        if (store === "universe") {
            return launcher.connect({ scope: scope ?? Scope.universe.id }, packageId, { elevate });
        }
        // connect to the registry the account's home serves, by its protocol
        else if (store === "registry") {
            const home = Account.home(await context.account());

            return launcher.connect({ space: home }, packageId, { elevate });
        }

        // connect to a named installation of the space at its origin, else to the space's own package where it is
        const space = await context.requireSpace();

        return launcher.connect(
            name === undefined ? { space } : { space, installation: name },
            packageId,
            { elevate },
        );
    }
}

/** The nouns and settings the stores of this machine, the universe and the space describe. */
export class Catalog {
    /** The nouns: the machine's, the universe's, then the space's. */
    readonly nouns: readonly CatalogEntry[];
    /** The settings the stores' installations declare. */
    readonly settings: readonly Setting[];

    /** Map words and identifiers to nouns, beside the declared settings. */
    constructor(nouns: readonly CatalogEntry[], settings: readonly Setting[]) {
        this.nouns = nouns;
        this.settings = settings;
    }

    /** Describe the stores of this machine, the universe once signed in, and the space in one. */
    static async of(context: CommandContext): Promise<Catalog> {
        // describe the command line's stores at once
        const isSignedIn = (await context.me()) !== undefined;
        const isSpace = isSignedIn && (await context.space()) !== undefined;
        const stores = STORES.filter(
            (store) =>
                store === "device" ||
                (store === "machine" || store === "universe" || store === "registry"
                    ? isSignedIn
                    : isSpace),
        );
        const descriptions = await Promise.all(
            stores.map(async (store) => ({
                store,
                description:
                    store === "registry"
                        ? Catalog.#registry(Account.home(await context.account()))
                        : await Catalog.describe(context, store),
            })),
        );

        // list a noun of each listed type, kept at the innermost store keeping its objects
        const nouns = descriptions.flatMap(({ store, description }) =>
            description.packages.flatMap((implementation) =>
                implementation.types.map(
                    (entry) => new CatalogEntry(implementation, store, entry, description),
                ),
            ),
        );
        const owned = nouns.filter((noun) => noun.isOwn);
        const innermost = owned.filter((noun) => {
            const same = owned.filter(
                (other) =>
                    other.entry.packageId === noun.entry.packageId &&
                    other.entry.type === noun.entry.type &&
                    other.qualifier === noun.qualifier,
            );

            return same.every((other) => STORES.indexOf(other.store) <= STORES.indexOf(noun.store));
        });

        // read the settings the packages declare
        const settings = descriptions.flatMap(({ description }) =>
            description.packages.flatMap((implementation) =>
                description
                    .declarations(implementation, SETTING_KIND)
                    .map((declared) => Setting.of(declared.description)),
            ),
        );

        return new Catalog(innermost, settings);
    }

    /** Describe the account's registry by its protocol's types, which every registry serves. */
    static #registry(home: string): ObjectStoreDescription {
        const types: ObjectType[] = Object.values(registryPackage.objects);
        const implementation = {
            packageId: registryPackage.id,
            scope: home,
            types: types.map((type) => ObjectTypeEntry.of(type)),
            declarations: [],
        };

        return new ObjectStoreDescription([implementation], ObjectTypePool.of(types));
    }

    /** Describe a store as the person or this machine's owner, knowing the built-in types. */
    static async describe(
        context: CommandContext,
        store: Exclude<Store, "registry">,
    ): Promise<ObjectStoreDescription> {
        // describe the store, the machine's partition by its scope beside the device's default one
        const connection = await CatalogEntry.connect(context, store, {
            packageId: DESCRIBING[store],
        });
        const known = BUILT_IN_PACKAGES.flatMap((handle) => Object.values(handle.objects)).filter(
            (type): type is ObjectType => type instanceof ObjectType,
        );

        return await ObjectStoreDescription.read(connection.store, {
            known,
            kinds: [ACTION_KIND, SETTING_KIND],
        });
    }

    /** Select an object by its identifier, in a space and a view. */
    async select(
        id: string,
        space: SpaceAddress,
        view: string | undefined,
        context: CommandContext,
    ): Promise<ObjectAddress> {
        // find the noun by the identifier's prefix, and open it
        const listed = this.owner(id);
        if (listed === undefined) {
            throw CliError.of("NOUN_NOT_FOUND", `noun listing objects like '${id}' not found`);
        }
        const noun = await listed.open(context);

        // read the object's scope through its type's get method
        const remote = await noun.client(context);
        const got = await remote.run(noun.type, noun.method("get"), { id });
        const row = schema.looseObject({ scope: schema.string() }).parse(got);

        return {
            space,
            object: {
                packageId: noun.type.policy.definition.packageId,
                type: noun.type.policy.name,
                scope: row.scope,
                id,
            },
            ...(view === undefined ? {} : { view }),
        };
    }

    /** Build the command of a word naming a noun: its plural lists, its singular gets and calls. */
    command(word: string): Command | undefined {
        // find the noun, refusing a word several nouns claim
        const entry = this.find(word);

        // leave a word no noun claims to the other commands
        if (entry === undefined) {
            return undefined;
        }
        // list a noun's objects by its plural, taking the singular's subcommands too
        else if (entry.lists(word)) {
            return {
                description: `List ${entry.plural} (${entry.qualifier})`,
                load: async (context) => {
                    const noun = await entry.open(context);

                    return { ...noun.command(), ...noun.list() };
                },
            };
        }
        // get, create and call a noun's objects by its singular
        else {
            return {
                description: `Get, create and change ${entry.plural} (${entry.qualifier})`,
                load: async (context) => (await entry.open(context)).command(),
            };
        }
    }

    /** Find the noun of a word, plural or singular and qualified or alone. */
    find(word: string): CatalogEntry | undefined {
        // refuse a word several nouns claim, listing their qualified words in order
        const claimants = this.claimants(word);
        const [only, ...others] = claimants;
        if (others.length > 0) {
            const qualified = claimants
                .map((noun) => noun.qualify(noun.lists(word) ? noun.plural : noun.name))
                .toSorted();
            throw CliError.of(
                "USAGE_INVALID",
                `noun '${word}' is invalid: it is ambiguous, so use one of '${qualified.join(", ")}'`,
            );
        }

        return only;
    }

    /** List the nouns a word refers to. */
    claimants(word: string): CatalogEntry[] {
        // take the one noun of a qualified word
        const qualified = this.nouns.find((noun) =>
            noun.forms.some((form) => word === noun.qualify(form)),
        );
        if (qualified !== undefined) {
            return [qualified];
        }

        return this.nouns.filter((noun) => noun.forms.includes(word));
    }

    /** Find the noun of an object by its identifier's prefix, the longest prefix first. */
    owner(id: string): CatalogEntry | undefined {
        const matching = this.nouns.filter((noun) => id.startsWith(`${noun.entry.identity}-`));

        return matching.toSorted(
            (left, right) => right.entry.identity.length - left.entry.identity.length,
        )[0];
    }

    /** Find the noun of an identity, such as machine: the type of that name among several. */
    identity(identity: string): CatalogEntry | undefined {
        // take the nouns of the identity, and the type it refers to among several
        const identified = this.nouns.filter((noun) => noun.entry.identity === identity);
        const owners = identified.filter((noun) => noun.entry.type === identity);
        const matching = identified.length > 1 && owners.length > 0 ? owners : identified;

        // refuse several types claiming it
        const [only, ...others] = matching;
        if (others.length > 0) {
            const words = matching.map((noun) => noun.qualify(noun.plural));
            throw CliError.of(
                "USAGE_INVALID",
                `'${identity}' is invalid: several nouns list it as '${words.join(", ")}'`,
            );
        }

        return only;
    }

    /** List the word listing each noun, qualified where another noun or a verb takes its word. */
    plurals(verbs: readonly string[]): string[] {
        return this.nouns.map((noun) => {
            const isAlone = this.claimants(noun.plural).length === 1;
            const isShadowed = noun.forms.some((form) => verbs.includes(form));

            return isAlone && !isShadowed ? noun.plural : noun.qualify(noun.plural);
        });
    }
}

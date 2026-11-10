import accountPackage from "@destack/account/package";
import machinePackage from "@destack/machine/package";
import updatePackage from "@destack/update/package";
import settingPackage from "@destack/setting/package";
import objectPackage from "@destack/object/package";
import type { ObjectAddress } from "@destack/view/declare";
import type { SpaceAddress } from "@destack/account/address";
import {
    type Command,
    type InstallationDescription,
    type MethodKind,
    ObjectType,
    type PackageHandle,
    Casing,
} from "@destack/object";
import {
    type ClientOptions,
    DynamicObjectClient,
    ObjectClient,
    ObjectStoreDescription,
} from "@destack/object/client";
import { PARTITION_HEADER } from "@destack/object/workload";
import { nth, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import stackPackage from "@destack/stack/package";
import { Setting, SETTING_KIND } from "@destack/setting";

import type { Context } from "../context/context.ts";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";

/** The separator of a qualified noun: the installation's name, then the noun. */
const QUALIFIER = ":";

/** The platform packages built into Destack, whose types the command line knows. */
const BUILT_IN_PACKAGES: readonly PackageHandle[] = [
    machinePackage,
    updatePackage,
    accountPackage,
    settingPackage,
    stackPackage,
    objectPackage,
];

/** The package each store describes itself at: a base installation in it. */
const DESCRIBING = {
    device: machinePackage.id,
    machine: machinePackage.id,
    universe: accountPackage.id,
    space: stackPackage.id,
} as const;

/** The stores the command line reads nouns from, outermost first. */
const STORES = ["device", "machine", "universe", "space"] as const;

/** The levels each store keeps objects at, the one it reads first for a type of any scope. */
const OWN_LEVELS = {
    device: ["device"],
    machine: ["machine"],
    universe: ["user", "account", Scope.universe.id],
    space: ["space"],
} as const satisfies Readonly<Record<(typeof STORES)[number], readonly string[]>>;

/** A store the command line reads nouns from: the device, machine, universe or space. */
export type Store = (typeof STORES)[number];

/** A noun of the command line: an object type at the installation listing it. */
export class Noun {
    /** The installation listing the type, as its store describes it. */
    readonly installation: InstallationDescription;
    /** The store running the installation. */
    readonly store: Store;
    /** The object type. */
    readonly type: ObjectType;
    /** The commands the installation declares on the type. */
    readonly commands: readonly Command[];

    /** Keep an object type at the installation listing it, in a store, with its commands. */
    constructor(
        installation: InstallationDescription,
        store: Store,
        type: ObjectType,
        commands: readonly Command[],
    ) {
        // keep the type, where it is listed, and its commands
        this.installation = installation;
        this.store = store;
        this.type = type;
        this.commands = commands;
    }

    /** Read the singular word. */
    get name(): string {
        return Casing.kebab(this.type.name);
    }

    /** Read the plural word, which lists. */
    get plural(): string {
        return Casing.kebab(this.type.plural);
    }

    /** Read the qualifier: the installation's name, else its package's name without the scope. */
    get qualifier(): string {
        const { name } = this.installation;
        const packageName = this.type.package.name;

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

    /** Find the name of the type's method of a method kind, refusing a type without one. */
    method(kind: MethodKind): string {
        // find the one method of the kind, refusing none and several
        const methods = Object.entries(this.type.describeMethods());
        const [found, other] = methods.filter(([, method]) => method.kind === kind);
        if (found === undefined) {
            throw CliError.of("METHOD_MISSING", `${this.plural} have no ${kind} method`);
        } else if (other !== undefined) {
            throw CliError.of("METHOD_AMBIGUOUS", `${this.plural} have several ${kind} methods`);
        }

        return found[0];
    }

    /** Find an object's identifier by its identifier or key, refusing keys of unkeyed types. */
    async identify(key: string, context: Context): Promise<string> {
        // take an identifier as given, and require a declared key for anything else
        if (this.type.identifies(key)) {
            return key;
        } else if (this.type.keyed === undefined) {
            throw CliError.of("USAGE_INVALID", `${this.plural} have no key: give the identifier`);
        }

        // get the object by its key, which its get method resolves
        const remote = await this.client(context);
        const row = await remote.call(this.type.name, this.method("get"), { id: key });

        return schema.looseObject({ id: schema.string() }).parse(row).id;
    }

    /** Open a client of the type's objects in the scope the terminal uses for them. */
    async client(context: Context): Promise<DynamicObjectClient> {
        return new DynamicObjectClient({
            objects: [this.type],
            scope: await this.#locate(context),
            endpoint: await Noun.options(context, this.store, this.installation),
            ...(await this.#homes(context)),
            environment: (await context.native()).environment,
        });
    }

    /** The levels of the scopes the type's objects live in, none for objects in any scope. */
    get levels(): string[] {
        return [this.type.scope]
            .flat()
            .map((scope) => (typeof scope === "string" ? scope : scope.name));
    }

    /** Whether the store keeps the objects at a level of its own, or they live in any scope. */
    get isOwn(): boolean {
        const own: readonly string[] = OWN_LEVELS[this.store];

        return this.levels.length === 0 || this.levels.some((level) => own.includes(level));
    }

    /** Locate the scope the type's objects live in for the person: the store's for several. */
    async #locate(context: Context): Promise<string> {
        // read the store's own level the type lives in, else its one level, any for a type of any scope
        const { levels } = this;
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
            return (await context.person()).person.subjectId;
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

    /** Read an installation's client options in a store, as the person or the machine's owner. */
    static async options(
        context: Context,
        store: Store,
        installation: Pick<InstallationDescription, "packageId" | "name">,
    ): Promise<ClientOptions> {
        // call this computer's device, and its machine's partition by the partition header
        const { packageId, name } = installation;
        const native = await context.native();
        if (store === "device") {
            return native.loopback.options(packageId);
        } else if (store === "machine") {
            const options = native.loopback.options(packageId);
            const send = present(options.fetch, "the loopback's fetch");
            const { machine } = await native.loopback.read();

            return {
                ...options,
                fetch: (request, ...rest) => {
                    request.headers.set(PARTITION_HEADER, machine);

                    return send(request, ...rest);
                },
            };
        }

        // call the universe at the person's issuer
        const person = await context.person();
        if (store === "universe") {
            return person.endpoint(packageId, "universe");
        }
        // call a named installation of the space at its origin
        else if (name !== undefined) {
            return person.installation(await context.requireSpace(), name, packageId);
        }
        // call a base installation of the space at the machine serving it
        else {
            return person.endpoint(packageId, await context.locate());
        }
    }

    /** Read the client options of the homes the type's changes act in, as the person. */
    async #homes(context: Context): Promise<{ readonly homes?: (scope: string) => ClientOptions }> {
        // call this computer's device or machine alone as its owner
        if (this.store === "device" || this.store === "machine") {
            return {};
        }

        // call each scope's home as the person
        const person = await context.person();

        return { homes: (scope) => person.store(scope) };
    }
}

/** The nouns and settings the stores of this machine, the universe and the space describe. */
export class Catalog {
    /** The nouns: the machine's, the universe's, then the space's. */
    readonly nouns: readonly Noun[];
    /** The settings the stores' installations declare. */
    readonly settings: readonly Setting[];

    /** Map words and identifiers to nouns, beside the declared settings. */
    constructor(nouns: readonly Noun[], settings: readonly Setting[]) {
        this.nouns = nouns;
        this.settings = settings;
    }

    /** Describe the stores of this machine, the universe once signed in, and the space in one. */
    static async of(context: Context): Promise<Catalog> {
        // describe the command line's stores at once
        const isSignedIn = (await context.me()) !== undefined;
        const isSpace = isSignedIn && (await context.space()) !== undefined;
        const stores = STORES.filter(
            (store) =>
                store === "device" ||
                (store === "machine" || store === "universe" ? isSignedIn : isSpace),
        );
        const described = await Promise.all(
            stores.map(async (store) => ({
                store,
                description: await Catalog.describe(context, store),
            })),
        );

        // make a noun of each listed type with its commands, kept at the innermost store keeping its objects
        const nouns = described.flatMap(({ store, description }) =>
            description.installations.flatMap((installation) => {
                const commands = description.commands(installation);

                return description.types(installation).map((type) => {
                    const own = commands.filter((command) => command.object === type);

                    return new Noun(installation, store, type, own);
                });
            }),
        );
        const owned = nouns.filter((noun) => noun.isOwn);
        const innermost = owned.filter((noun) => {
            const same = owned.filter(
                (other) =>
                    other.type.package.id === noun.type.package.id &&
                    other.type.name === noun.type.name &&
                    other.qualifier === noun.qualifier,
            );

            return same.every((other) => STORES.indexOf(other.store) <= STORES.indexOf(noun.store));
        });

        // read the settings the installations declare
        const settings = described.flatMap(({ description }) =>
            description.installations.flatMap((installation) =>
                description
                    .declarations(installation, SETTING_KIND)
                    .map((declared) => Setting.of(declared.description)),
            ),
        );

        return new Catalog(innermost, settings);
    }

    /** Describe a store as the person or this machine's owner, knowing the built-in types. */
    static async describe(context: Context, store: Store): Promise<ObjectStoreDescription> {
        // describe the store, the machine's partition by its scope beside the device's default one
        const client = await context.native();
        const options = await Noun.options(context, store, { packageId: DESCRIBING[store] });
        const known = BUILT_IN_PACKAGES.flatMap((handle) => Object.values(handle.objects)).filter(
            (type): type is ObjectType => type instanceof ObjectType,
        );

        return await ObjectStoreDescription.read(
            ObjectClient.store(options, client.environment),
            known,
        );
    }

    /** Select an object by its identifier, in a space and a view. */
    async select(
        id: string,
        space: SpaceAddress,
        view: string | undefined,
        context: Context,
    ): Promise<ObjectAddress> {
        // find the noun by the identifier's prefix
        const noun = this.owner(id);
        if (noun === undefined) {
            throw CliError.of("NOUN_NOT_FOUND", `nothing lists objects like ${id}`);
        }

        // read the object's scope through its type's get method
        const remote = await noun.client(context);
        const got = await remote.call(noun.type.name, noun.method("get"), { id });
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

    /** Find the noun of a word, plural or singular and qualified or alone. */
    find(word: string): Noun | undefined {
        // refuse a word several nouns claim, listing their qualified words in order
        const claimants = this.claimants(word);
        const [only, ...others] = claimants;
        if (others.length > 0) {
            const qualified = claimants
                .map((noun) => noun.qualify(noun.lists(word) ? noun.plural : noun.name))
                .toSorted();
            throw CliError.of(
                "USAGE_INVALID",
                `${word} is ambiguous: use one of ${qualified.join(", ")}`,
            );
        }

        return only;
    }

    /** List the nouns a word refers to. */
    claimants(word: string): Noun[] {
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
    owner(id: string): Noun | undefined {
        const matching = this.nouns.filter((noun) => id.startsWith(`${noun.type.identity}-`));

        return matching.toSorted(
            (left, right) => right.type.identity.length - left.type.identity.length,
        )[0];
    }

    /** Find the noun of an identity, such as machine: the type of that name among several. */
    identity(identity: string): Noun | undefined {
        // take the nouns of the identity, and the type it refers to among several
        const identified = this.nouns.filter((noun) => noun.type.identity === identity);
        const owners = identified.filter((noun) => noun.type.name === identity);
        const matching = identified.length > 1 && owners.length > 0 ? owners : identified;

        // refuse several types claiming it
        const [only, ...others] = matching;
        if (others.length > 0) {
            const words = matching.map((noun) => noun.qualify(noun.plural));
            throw CliError.of(
                "USAGE_INVALID",
                `several nouns list ${identity}s: ${words.join(", ")}`,
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

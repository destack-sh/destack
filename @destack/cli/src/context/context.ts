import { spaceAliases } from "@destack/account/declare";
import { currentSpace, type Login, NATIVE_SETTINGS } from "@destack/account/native";
import type { Environment } from "@destack/environment";
import { dirname, relative } from "node:path";
import { DESKTOP_SETTINGS } from "@destack/desktop/setting";
import { type ObjectType, type PackageHandle } from "@destack/object";
import { type ObjectClient, createClient } from "@destack/object/client";
import { Setting } from "@destack/setting";
import type { PlacementEndpoint } from "@destack/account/object";
import { type Session, type Device, type MachineCredential } from "@destack/machine/device";
import { checkout, type Checkout } from "@destack/machine/checkout";
import { RequestId } from "@destack/sync";
import { type Identifier, schema, Version } from "@destack/schema";

import { Catalog } from "../object/catalog.ts";
import { Terminal } from "../output/index.ts";
import type { Decision, GrantRequest } from "@destack/account/authentication";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";

/** The localization the terminal reads permissions in: the source language. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

/** The settings Destack's packages declare beside the space's installations'. */
const BUILT_IN_SETTINGS: readonly Setting[] = [...NATIVE_SETTINGS, ...DESKTOP_SETTINGS];

/** The environment capabilities a command line runs with. */
export type CliEnvironment = Environment<"files" | "process" | "host" | "opener" | "server">;

/** What a command line acts in: its environment, this computer's device, the person and a space. */
export class Context {
    /** The space --space names: an address, identifier, alias, home, residency or bare name. */
    readonly spaceOption: string | undefined;
    /** The command line's clock, network, files, processes and platform. */
    readonly environment: CliEnvironment;
    /** The terminal the command line writes its output and diagnostics to. */
    readonly terminal: Terminal;
    /** Open this computer's native device, which its machine created at its first start. */
    readonly #open: () => Promise<Device<MachineCredential>>;
    /** This computer's native device, opened on first use. */
    #native: Promise<Device<MachineCredential>> | undefined;
    /** The login the terminal acts as, read once. */
    #login: Promise<Login["row"] | undefined> | undefined;
    /** The person signed in, read once. */
    #person: Promise<Session> | undefined;
    /** The space the command line acts in, read once. */
    #space: Promise<Identifier<"space"> | undefined> | undefined;
    /** Where the space lives, found once. */
    #location: Promise<PlacementEndpoint> | undefined;
    /** The nouns and settings the stores describe, read once. */
    #catalog: Promise<Catalog> | undefined;

    /** Act in an environment through the native device an opener opens, in the space of --space. */
    constructor(
        open: () => Promise<Device<MachineCredential>>,
        space: string | undefined,
        environment: CliEnvironment,
    ) {
        // set the opener, the space, the environment and its terminal
        this.#open = open;
        this.spaceOption = space;
        this.environment = environment;
        this.terminal = new Terminal(environment.process, environment.timeZone);
    }

    /** Open this computer's native device once. */
    native(): Promise<Device<MachineCredential>> {
        this.#native ??= this.#open();

        return this.#native;
    }

    /** Read the person signed in on this machine, as `@me` reads, absent before anyone signs in. */
    async me(): Promise<string | undefined> {
        return (await this.login())?.subjectId;
    }

    /** Read the login the terminal acts as, once, absent before anyone signs in. */
    login(): Promise<Login["row"] | undefined> {
        this.#login ??= this.native().then((native) => native.login());

        return this.#login;
    }

    /** List the settings Destack's packages and the space's installations declare. */
    async settings(): Promise<Setting[]> {
        return [...BUILT_IN_SETTINGS, ...(await this.catalog()).settings];
    }

    /** Read the person the terminal acts as, refusing a computer nobody signed in on. */
    person(): Promise<Session> {
        this.#person ??= (async () => (await this.native()).session(await this.#loginId()))();

        return this.#person;
    }

    /** Read the login the terminal acts through, refusing a computer nobody signed in on. */
    async signedIn(): Promise<Login> {
        return (await this.native()).signedIn(await this.#loginId());
    }

    /** Read the login the terminal acts as by its identifier, refusing none signed in. */
    async #loginId(): Promise<Identifier<"login">> {
        // require a person signed in on this computer
        const signedIn = await this.login();
        if (signedIn === undefined) {
            throw CliError.of("SIGN_IN_REQUIRED", "sign in with destack login");
        }

        return signedIn.id;
    }

    /** Read the space the command line acts in: --space, else this computer's space setting. */
    space(): Promise<Identifier<"space"> | undefined> {
        this.#space ??= (async () => {
            // resolve the space --space names
            if (this.spaceOption !== undefined) {
                return this.resolve(this.spaceOption);
            }

            // take this computer's space setting, absent without one
            const set = await (await this.native()).setting(currentSpace);

            return set ?? undefined;
        })();

        return this.#space;
    }

    /** Resolve a space's name, as the signed-in person names it. */
    async resolve(name: string): Promise<Identifier<"space">> {
        const aliases = await (await this.native()).setting(spaceAliases);

        return (await this.person()).resolve(name, aliases);
    }

    /** Read the space the command line acts in, refusing a command line acting in none. */
    async requireSpace(): Promise<Identifier<"space">> {
        const space = await this.space();
        if (space === undefined) {
            throw CliError.of(
                "SPACE_REQUIRED",
                "set the space with --space <space>, or switch to one with destack switch <space>",
            );
        }

        return space;
    }

    /** Find where the space the command line acts in lives, as the person signed in, once. */
    locate(): Promise<PlacementEndpoint> {
        this.#location ??= (async () => (await this.native()).locate(await this.requireSpace()))();

        return this.#location;
    }

    /** Read the account the person acts in: the space's, else their own. */
    async account(): Promise<Identifier<"account">> {
        // read the person's own account outside a space
        const space = await this.space();
        if (space === undefined) {
            return (await this.person()).person.account();
        }

        // read the account of the space
        const location = await this.locate();

        return schema.identifier("account").parse(location.placement.scope);
    }

    /** Read the residency home serving finance for the account, else for the person's account. */
    async residency(): Promise<Identifier<"space">> {
        const person = await this.person();

        return await person.residency(await this.account());
    }

    /** Open some local object types of a package as this computer's client, closed with a stack. */
    async objects<const Objects extends Readonly<Record<string, ObjectType>>>(
        objects: Objects,
        stack: AsyncDisposableStack,
    ): Promise<ObjectClient<Objects>> {
        // open the objects, closed with the stack
        const client = await this.native();
        const opened = await client.objects(objects);
        stack.defer(() => opened.close());

        return opened;
    }

    /** Open a client of a package of the machine serving a space, as the person. */
    async package<Handle extends PackageHandle>(handle: Handle, location: PlacementEndpoint) {
        // send as the person, asking them at the terminal for a consent a call needs
        const client = await this.native();
        const session = await this.person();
        const decide = (request: GrantRequest) => this.#decide(request);

        return createClient<Handle["router"]>(
            handle,
            session.endpoint(handle.id, location, { decide }),
            client.environment,
        );
    }

    /** Ask the person at the terminal whether to grant what an installation asks. */
    async #decide(request: GrantRequest): Promise<Decision> {
        // name the package, its address if any, its space and each permission as its package describes it
        const named = request.installation === undefined ? "" : ` (${request.installation})`;
        const permissions = request.permissions.map(
            (permission) => `  - ${SOURCE.render(permission.description)}`,
        );
        const question = [
            `${request.package}${named} in ${request.space} asks to:`,
            ...permissions,
            "Allow it?",
        ].join("\n");

        return (await this.terminal.prompt.confirm(question)) ? "approve" : "deny";
    }

    /** Register a directory's working tree unless a checkout has it, and read its path there. */
    async registerCheckout(
        directory: string,
    ): Promise<{ readonly checkout: Checkout; readonly path: string }> {
        // list the canonical directory and its ancestors
        const canonical = await this.environment.files.realPath(directory);
        let ancestor = canonical;
        const ancestors = [ancestor];
        while (dirname(ancestor) !== ancestor) {
            ancestor = dirname(ancestor);
            ancestors.push(ancestor);
        }

        // query the checkouts registered at any of them
        const client = await this.native();
        await using opened = new AsyncDisposableStack();
        const checkouts = await client.follow({ checkout }, opened);
        const rows = await checkouts.query.checkout.findMany({
            where: { directory: { in: ancestors } },
        });
        const [innermost] = rows.toSorted(
            (left, right) => right.directory.length - left.directory.length,
        );

        // take the innermost, else register the directory's existing working tree
        const found =
            innermost ??
            (await client.machine().checkout.create({
                machineId: (await client.loopback.computer()).machineId,
                requestId: RequestId.create(client.environment),
                directory: canonical,
                source: { kind: "existing" },
            }));

        // read the directory's path relative to the checkout's root, `.` for the root
        const inside = relative(found.root, canonical);

        return { checkout: found, path: inside === "" ? "." : inside };
    }

    /** Find the setting of a name or a package-qualified name, at its latest release. */
    async setting(name: string): Promise<Setting> {
        // read the settings Destack's packages and the space's installations declare
        const settings = await this.settings();

        // match the name, qualified by the package's name when it has a slash
        const slash = name.lastIndexOf("/");
        const owner = slash < 0 ? undefined : name.slice(0, slash);
        const matching = settings
            .filter((entry) => entry.name === name.slice(slash + 1))
            .filter((entry) => owner === undefined || entry.package.name === owner)
            .toSorted((left, right) =>
                Version.compare(right.package.version, left.package.version),
            );
        const packages = [...new Set(matching.map((entry) => entry.package.name))];
        const [latest] = matching;

        // refuse a name nothing declares
        if (latest === undefined) {
            throw CliError.of("SETTING_NOT_DECLARED", `setting ${name} is not declared here`);
        }
        // refuse a name several packages declare
        else if (packages.length > 1) {
            const qualified = packages.map((entry) => `${entry}/${latest.name}`);
            throw CliError.of(
                "SETTING_AMBIGUOUS",
                `${name} is ambiguous: use one of ${qualified.join(", ")}`,
            );
        }

        return latest;
    }

    /** Read the nouns and settings the stores describe, once. */
    catalog(): Promise<Catalog> {
        this.#catalog ??= Catalog.of(this);

        return this.#catalog;
    }
}

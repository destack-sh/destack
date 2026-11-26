import type { Login } from "@destack/account/native";
import type { Environment } from "@destack/environment";
import { type ObjectType, type Package } from "@destack/object";
import type { ClientOf } from "@destack/object/client";
import type { ObjectClient } from "@destack/object/device";
import type registryPackage from "@destack/registry/package";
import { PackageName } from "@destack/package";
import type { Setting } from "@destack/setting";
import { Account, type PlacementEndpoint } from "@destack/account/object";
import {
    type Launcher,
    type LauncherConsent,
    type Device,
    type MachineCredential,
} from "@destack/machine/device";
import { Contact, principal, type StepUp } from "@destack/access";
import { type Identifier, schema, Version } from "@destack/schema";

import type { Catalog } from "../catalog/catalog.ts";
import { Terminal } from "../terminal/index.ts";
import type { Decision, GrantRequest } from "@destack/account/authentication";
import type {} from "@destack/package/import-meta";
import { type ObjectReference, Scope } from "@destack/sync";
import { CliError } from "../error/index.ts";

/** The prefix of a person's handle, as people write it. */
const HANDLE_PREFIX = "@";

/** The environment capabilities a command line runs with. */
export type CommandEnvironment = Environment<"files" | "process" | "host" | "opener" | "server">;

/** What a command line acts in: its environment, this computer's device, the person and a space. */
export class CommandContext implements AsyncDisposable {
    /** The space --space names: an address, identifier, alias, home, region or bare name. */
    readonly spaceOption: string | undefined;
    /** The command line's clock, network, files, processes and platform. */
    readonly environment: CommandEnvironment;
    /** The terminal the command line writes its output and diagnostics to. */
    readonly terminal: Terminal;
    /** Open this computer's native device, which its machine created at its first start. */
    readonly #open: () => Promise<Device<MachineCredential>>;
    /** This computer's native device, opened on first use. */
    #native: Promise<Device<MachineCredential>> | undefined;
    /** The login the terminal acts as, read once. */
    #login: Promise<Login["row"] | undefined> | undefined;
    /** The launcher of the person signed in, opened once. */
    #launcher: Promise<Launcher> | undefined;
    /** The space the command line acts in, read once. */
    #space: Promise<Identifier<"space"> | undefined> | undefined;
    /** Where the space lives, found once. */
    #location: Promise<PlacementEndpoint> | undefined;
    /** The nouns and settings the stores describe, read once. */
    #catalog: Promise<Catalog> | undefined;
    /** What closes once the command line ran, such as its connections. */
    readonly #closing = new AsyncDisposableStack();

    /** Act in an environment through the native device an opener opens, in the space of --space. */
    constructor(
        open: () => Promise<Device<MachineCredential>>,
        space: string | undefined,
        environment: CommandEnvironment,
    ) {
        // set the opener, the space, the environment and its terminal
        this.#open = open;
        this.spaceOption = space;
        this.environment = environment;
        this.terminal = new Terminal(environment.process, environment);
    }

    /** Close something once the command line ran, as its connections close. */
    defer(close: () => void | Promise<void>): void {
        this.#closing.defer(close);
    }

    /** Close what the command line opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#closing.disposeAsync();
    }

    /** Import a module as one operation of real work, which the environment's scheduler orders. */
    module<Module>(load: () => Promise<Module>): Promise<Module> {
        return this.environment.scheduler.run(load);
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
        // take the native and desktop settings Destack declares, then the space's installations'
        const { NATIVE_SETTINGS } = await this.module(() => import("@destack/account/native"));
        const { DESKTOP_SETTINGS } = await this.module(() => import("@destack/desktop/setting"));

        return [...NATIVE_SETTINGS, ...DESKTOP_SETTINGS, ...(await this.catalog()).settings];
    }

    /** Open the launcher of the person the terminal acts as, refusing one nobody signed in on. */
    launcher(): Promise<Launcher> {
        this.#launcher ??= (async () => (await this.native()).launcher(await this.#loginId()))();

        return this.#launcher;
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
            throw CliError.of(
                "SIGN_IN_UNAUTHENTICATED",
                "authentication required: sign in with destack login",
            );
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
            const { currentSpace } = await this.module(() => import("@destack/account/native"));
            const set = await (await this.native()).setting(currentSpace);

            return set ?? undefined;
        })();

        return this.#space;
    }

    /** Resolve a space's name, as the signed-in person names it. */
    async resolve(name: string): Promise<Identifier<"space">> {
        const { spaceAliases } = await this.module(() => import("@destack/account/declare"));
        const aliases = await (await this.native()).setting(spaceAliases);

        return (await this.launcher()).resolve(name, aliases);
    }

    /** Read the space the command line acts in, refusing a command line acting in none. */
    async requireSpace(): Promise<Identifier<"space">> {
        const space = await this.space();
        if (space === undefined) {
            throw CliError.of(
                "SPACE_REQUIRED",
                "space is required: set it with --space <space>, or switch to one with destack switch <space>",
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
            return (await this.launcher()).account();
        }

        // read the account of the space
        const location = await this.locate();

        return schema.identifier("account").parse(location.placement.scope);
    }

    /** Read the principal a person writes: a handle's user, an email's contact, or a user. */
    async subject(named: string): Promise<ObjectReference> {
        // take a handle's person, an email's contact, else a user's identifier
        const email = schema.email().safeParse(named);
        const user = schema.identifier("user").safeParse(named);
        if (named.startsWith(HANDLE_PREFIX)) {
            const native = await this.native();
            const accountId = await native.resolve(named.slice(HANDLE_PREFIX.length));

            return principal.user.reference(Scope.universe.id, Account.person(accountId));
        } else if (email.success) {
            return Contact.subject(Contact.email(email.data));
        } else if (user.success) {
            return principal.user.reference(Scope.universe.id, user.data);
        }

        // refuse other text
        throw CliError.of(
            "USAGE_INVALID",
            `person '${named}' is invalid: give a @handle, an email or a user identifier`,
        );
    }

    /** Open the registry of a package name's account, in its region's home, as the person. */
    async registry(name: string): Promise<{
        /** The registry's client. */
        readonly client: ClientOf<typeof registryPackage>;
        /** The region's home serving the registry. */
        readonly home: Identifier<"space">;
        /** The account of the name's handle. */
        readonly accountId: Identifier<"account">;
    }> {
        // find the account of the name's handle and its home, whose forge serves its packages
        const native = await this.native();
        const accountId = await native.resolve(PackageName.split(name).handle);
        const home = Account.home(accountId);

        // call the registry the home serves
        const { createClient } = await this.module(() => import("@destack/object/client"));
        const { default: registryPackage } = await this.module(
            () => import("@destack/registry/package"),
        );
        const launcher = await this.launcher();
        const endpoint = await launcher.package(home, registryPackage.id);

        return {
            client: createClient(registryPackage, endpoint, native.environment),
            home,
            accountId,
        };
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
    async package<Handle extends Package>(
        handle: Handle,
        location: PlacementEndpoint,
        consent: "ask" | "approve" = "ask",
    ) {
        // send as the person, answering at the terminal what a call asks of them
        const { createClient } = await this.module(() => import("@destack/object/client"));
        const client = await this.native();
        const session = await this.launcher();

        return createClient<Handle["router"]>(
            handle,
            session.endpoint(handle.id, location, this.consent(consent)),
            client.environment,
        );
    }

    /** Answer what a call asks of the person: a grant at the terminal, a step-up in the browser. */
    consent(grant: "ask" | "approve" = "ask"): Required<LauncherConsent> {
        return {
            decide: async (request) => (grant === "approve" ? "approve" : this.#decide(request)),
            elevate: (stepUp) => this.#elevate(stepUp),
        };
    }

    /** Sign the person in again in the browser as a step-up asks, and read their launcher. */
    async #elevate(stepUp: StepUp): Promise<Launcher> {
        // tell the person why the browser opens, then sign them in again there
        const { terminal, environment } = this;
        await terminal.printDiagnostic(
            terminal.event("Waiting", "running", "for you to confirm it is you"),
        );
        const native = await this.native();
        const elevated = await native.signIn({ kind: "browser", stepUp }, async (state) => {
            if (state.kind === "open") {
                await terminal.printDiagnostic(terminal.event("Opening", "running", state.url));
                await environment.opener.openUrl(state.url);
            }
        });

        // act through the launcher of the stepped-up sign-in from here on
        this.#launcher = native.launcher(elevated.id);

        return this.#launcher;
    }

    /** Ask the person at the terminal whether to grant what an installation asks. */
    async #decide(request: GrantRequest): Promise<Decision> {
        // name the package, its address if any, its space and each permission as its package describes it
        const { Localization, SOURCE_LOCALE } = await this.module(() => import("@destack/locale"));
        const source = Localization.of(SOURCE_LOCALE, []);
        const named = request.installation === undefined ? "" : ` (${request.installation})`;
        const permissions = request.permissions.map(
            (permission) => `  - ${source.render(permission.description)}`,
        );
        const question = [
            `${request.package}${named} in ${request.space} asks to:`,
            ...permissions,
            "Allow it?",
        ].join("\n");

        return (await this.terminal.prompt.confirm(question)) ? "approve" : "deny";
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
            throw CliError.of(
                "SETTING_NOT_FOUND",
                `setting '${name}' not found: it is not declared here`,
            );
        }
        // refuse a name several packages declare
        else if (packages.length > 1) {
            const qualified = packages.map((entry) => `${entry}/${latest.name}`);
            throw CliError.of(
                "SETTING_INVALID",
                `setting '${name}' is invalid: it is ambiguous, so use one of '${qualified.join(", ")}'`,
            );
        }

        return latest;
    }

    /** Read the nouns and settings the stores describe, once. */
    catalog(): Promise<Catalog> {
        this.#catalog ??= this.module(() => import("../catalog/catalog.ts")).then(({ Catalog }) =>
            Catalog.of(this),
        );

        return this.#catalog;
    }
}

import { inspect } from "node:util";
import { argument, LocaleTag, Localization, t } from "@destack/locale";
import type { Select } from "@destack/db";
import type { Environment } from "@destack/environment";
import type { SpaceAddress } from "@destack/account/address";
import { currentLogin, type Login } from "@destack/account/native";
import { MachineProcess } from "@destack/machine/computer";
import { checkForUpdates, distribution, restartToUpdate } from "@destack/update/object";
import { Stage } from "@destack/update/release";
import { computer, Device } from "@destack/machine/device";
import { type ObjectClient, RelationalQuery } from "@destack/object/client";
import { schema } from "@destack/schema";
import { FOCUS_PARAMETER, start } from "@destack/view/declare";
import { DESKTOP_OBJECTS, window } from "../object/index.ts";
import { Autostart, Menu, type MenuEntry } from "../server/index.ts";
import { type Event, Shell } from "../shell/index.ts";
import { DesktopLock } from "./lock.ts";
import { MACHINE_ENTRY } from "./machine.ts";
import { Person } from "./person.ts";
import { Link } from "../link/index.ts";
import { DesktopError } from "../error/index.ts";

/** The view of Home showing the person's settings, which the settings window opens. */
const SETTINGS_VIEW = "settings";

/** The host capabilities the desktop runs with: its files, processes, platform and server. */
export type DesktopHost = Environment<"files" | "process" | "host" | "server">;

/** Who the desktop acts as: a signed-in person, or nobody while signing in or signed out. */
export type SessionStatus =
    | { readonly status: "signedIn"; readonly login: Pick<Login["row"], "name"> }
    | { readonly status: "signingIn" | "signedOut" };

/** What the palette floats over: a space, and the link of the window in focus there. */
interface Focus {
    /** The space of the window the palette floats over. */
    readonly space: SpaceAddress;
    /** The link of the window the palette floats over, absent over none. */
    readonly link?: string;
}

/** This computer's running desktop: its menus, updates, autostart and the person signed in. */
export class Desktop implements AsyncDisposable {
    /** The native device of this computer. */
    readonly native: Device;
    /** The native shell. */
    readonly shell: Shell;
    /** The desktop's local objects: its windows and the objects people opened. */
    readonly objects: ObjectClient<typeof DESKTOP_OBJECTS>;
    /** The host's items in the tray and the application menu. */
    readonly menu: Menu;
    /** The desktop's registration at login. */
    readonly autostart: Autostart;
    /** This machine's distribution, which the update commands act on. */
    readonly #distributions: ObjectClient<{ readonly distribution: typeof distribution }>;
    /** This computer's record followed from its machine, with the enrollment a person needs. */
    readonly computers: ObjectClient<{ readonly computer: typeof computer }>;
    /** The links of this stage's desktop. */
    readonly links: Link;
    /** The host's language the desktop shows its menus in, as native applications do. */
    readonly localization: Localization;
    /** The host the desktop runs on, whose standard error takes failures before a sign-in. */
    readonly environment: DesktopHost;
    /** Aborts once the desktop stops. */
    readonly #stopping: AbortController;
    /** The clients the desktop opened, closed once it stops. */
    readonly #opened: AsyncDisposableStack;
    /** The person signed in, absent while nobody is. */
    #person: Person | undefined;
    /** The browser sign-in running and what stops it, absent while none runs. */
    #signingIn: { readonly done: Promise<void>; readonly stopping: AbortController } | undefined;

    /** Run over the native device, the shell, the windows, the clients and the links. */
    private constructor(
        native: Device,
        shell: Shell,
        objects: ObjectClient<typeof DESKTOP_OBJECTS>,
        distributions: ObjectClient<{ readonly distribution: typeof distribution }>,
        computers: ObjectClient<{ readonly computer: typeof computer }>,
        links: Link,
        environment: DesktopHost,
    ) {
        // stop nothing yet
        this.#stopping = new AbortController();
        this.#opened = new AsyncDisposableStack();

        // set the native device, the shell, the windows and the links
        this.native = native;
        this.shell = shell;
        this.objects = objects;
        this.links = links;

        // read the host's language without its extensions
        const locale = new Intl.Locale(environment.locale).baseName;
        this.localization = Localization.of(LocaleTag.parse(locale), []);

        // fill the menus, with the update commands and the autostart toggle
        this.menu = new Menu(shell, this.localization);
        this.#distributions = distributions;
        this.computers = computers;
        this.autostart = new Autostart(native, this.menu, shell, this.localization);

        // read every fact of the host from its environment
        this.environment = environment;
    }

    /** Start this computer's machine and run the desktop until the shell closes its pipe. */
    static async start(
        environment: DesktopHost,
        shell = Shell.of(environment.process),
    ): Promise<void> {
        // start this computer's machine and run as this user's one desktop
        await MachineProcess.current(environment).start(MACHINE_ENTRY);
        await using native = await Device.current(environment);
        await using lock = await new DesktopLock(native.home, environment).acquire();
        const links = Link.of(Stage.current(environment));
        await using desktop = await Desktop.open(native, shell, links, environment);

        // open the link the desktop launched with
        const [link] = environment.process.arguments;
        await desktop.run(link);
    }

    /** Open the desktop over this computer's native device and its machine. */
    static async open(
        native: Device,
        shell: Shell,
        links: Link,
        environment: DesktopHost,
    ): Promise<Desktop> {
        // open the windows as the native device, and follow the machine's distribution and record
        await using opened = new AsyncDisposableStack();
        const objects = await native.objects(DESKTOP_OBJECTS);
        const distributions = await native.follow({ distribution }, opened);
        const computers = await native.follow({ computer }, opened);

        // close the clients with the desktop
        const desktop = new Desktop(
            native,
            shell,
            objects,
            distributions,
            computers,
            links,
            environment,
        );
        desktop.#opened.use(opened.move());

        return desktop;
    }

    /** List the account section's entries for who the desktop acts as. */
    static accountEntries(
        session: SessionStatus,
        desktop: Pick<Desktop, "signIn" | "signOut" | "switch" | "localization">,
    ): MenuEntry[] {
        const { localization } = desktop;

        // offer signing out or switching to the person signed in
        if (session.status === "signedIn") {
            return [
                { kind: "label", title: session.login.name },
                {
                    kind: "action",
                    id: "switch",
                    title: localization.render(t`Switch Account…`),
                    run: () => desktop.switch(),
                },
                {
                    kind: "action",
                    id: "signOut",
                    title: localization.render(t`Sign Out`),
                    run: () => desktop.signOut(),
                },
            ];
        }
        // show the sign-in running
        else if (session.status === "signingIn") {
            return [{ kind: "label", title: localization.render(t`Signing In…`) }];
        }
        // offer signing in to nobody
        else {
            return [
                {
                    kind: "action",
                    id: "signIn",
                    title: localization.render(t`Sign In…`),
                    run: () => desktop.signIn(),
                },
            ];
        }
    }

    /** Run until the shell closes its pipe: sign in, open a link and answer each event. */
    async run(link: string | undefined): Promise<void> {
        // title the shell's built-in menu items in the host's language
        await this.shell.send({
            action: "menu.title",
            start: this.localization.render(t`New Window`),
            quit: this.localization.render(t`Quit`),
        });

        // follow the person signed in, the update commands and the autostart toggle
        const { signal } = this.#stopping;
        const following = Promise.all([
            this.#followLogin(link, signal).catch((error: unknown) =>
                this.report("login.failed", error),
            ),
            this.autostart
                .follow(signal)
                .catch((error: unknown) => this.report("autostart.failed", error)),
            this.#followUpdates(signal).catch((error: unknown) =>
                this.report("update.failed", error),
            ),
        ]);

        // sign in while nobody is, then answer each event until the shell closes its pipe
        try {
            await this.signIn();
            for await (const event of this.shell.events()) {
                await this.dispatch(event).catch((error: unknown) =>
                    this.report(`${event.action}.failed`, error),
                );
            }
        } finally {
            this.#stopping.abort();
            await Promise.all([this.#signingIn?.done, following]);
        }
    }

    /** Start a browser sign-in unless a person is signed in or one runs, which the follow shows. */
    async signIn(): Promise<void> {
        // leave a person signed in here or by another program, and a sign-in that runs
        const { native, shell } = this;
        if (this.#signingIn !== undefined || (await native.login()) !== undefined) {
            return;
        }

        // sign in through the browser until a person shows or the desktop stops
        await this.menu.set("account", Desktop.accountEntries({ status: "signingIn" }, this));
        const stopping = new AbortController();
        const signal = AbortSignal.any([this.#stopping.signal, stopping.signal]);
        const signedIn = native.signIn(
            async (url) => shell.send({ action: "browser.open", url }),
            signal,
        );

        // report a failure and offer signing in again, unless the desktop stopped the sign-in
        const done = signedIn
            .then(
                () => undefined,
                async (error: unknown) => {
                    if (!signal.aborted) {
                        this.report("sign-in.failed", error);
                        await this.menu.set(
                            "account",
                            Desktop.accountEntries({ status: "signedOut" }, this),
                        );
                    }
                },
            )
            .finally(() => {
                this.#signingIn = undefined;
            });
        this.#signingIn = { done, stopping };
    }

    /** Sign the person out on this computer, whom the login follow then clears from the screen. */
    async signOut(): Promise<void> {
        // send the person's telemetry while their session stands, then revoke this computer's client
        const person = this.#requirePerson();
        await person.telemetry.flush();
        await this.native.signOut(person.session.login);
    }

    /** Sign the person out and another person in. */
    async switch(): Promise<void> {
        await this.signOut();
        await this.signIn();
    }

    /** Answer one event of the shell. */
    async dispatch(event: Event): Promise<void> {
        // report a failure of the shell
        if (event.action === "shell.failed") {
            this.report(`${event.command}.failed`, new Error(event.message));
        }
        // run the menu item the person chose
        else if (event.action === "menu.chosen") {
            await this.menu.choose(event.item);
        }
        // open a link another application opened, or the start link
        else if (event.action === "link.opened") {
            await this.openLink(event.url);
        }
        // bring back the window the person returns to, signing in while nobody is
        else if (event.action === "application.reopened") {
            await (this.#person === undefined ? this.signIn() : this.reopen());
        }
        // let the person answer every other event
        else {
            await this.#requirePerson().answer(event);
        }
    }

    /** Open a link in a window of this computer, or the person's start link for null. */
    async openLink(url: string | null): Promise<void> {
        // open it as a window row, which the person's windows show
        const link = url ?? (await this.#startLink(this.#requirePerson()));
        await this.#windows().open({ link }).confirmed;
    }

    /** Bring the window the person returns to into focus, or open the start link without one. */
    async reopen(): Promise<void> {
        // open the start link without a window to return to
        const front = this.#requirePerson().front();
        if (front === undefined) {
            await this.openLink(null);
        }
        // ask for the window's focus, which the person's windows give it
        else {
            await this.#windows().focus({ id: front.id }).confirmed;
        }
    }

    /** Tell the person something about the desktop in a banner, under a notice's key. */
    async notify(key: string, title: string, body: string): Promise<void> {
        await this.shell.send({
            action: "banner.show",
            id: `notice.${key}`,
            title,
            body,
            interruption: "active",
            actions: [],
        });
    }

    /** Open Home's settings view of the person in a window. */
    async openSettings(): Promise<void> {
        // open it as a window row
        const view = await this.#requirePerson().session.view("home");
        await this.#windows().open({ link: this.links.format({ ...view, view: SETTINGS_VIEW }) })
            .confirmed;
    }

    /** Float the command palette over a space, the focused window's by default, as a window row. */
    async openPalette(over?: Focus): Promise<void> {
        // tell the person when there is no space to run commands in
        const person = this.#requirePerson();
        const focus = over ?? (await this.focus());
        if (focus === undefined) {
            await this.notify(
                "palette",
                this.localization.render(t`No space to run commands in`),
                this.localization.render(t`Open a view, or set the desktop's start link.`),
            );
        }
        // open Home's command view over the window's link, replacing a palette that floats
        else {
            const path =
                focus.link === undefined
                    ? "/"
                    : `/?${FOCUS_PARAMETER}=${encodeURIComponent(focus.link)}`;
            const view = await person.session.view("command", path);
            await person.closePalettes();
            await this.#windows().open({ link: this.links.format(view), kind: "palette" })
                .confirmed;
        }
    }

    /** Read what the palette opens over: the focused window, else the start link. */
    async focus(): Promise<Focus | undefined> {
        // take the focused window's
        const person = this.#requirePerson();
        const focused = person.inFocus();
        if (focused !== undefined) {
            return { space: focused.address.space, link: focused.link };
        }

        // take the start link's space, none for a link to no view or object
        const address = this.links.parse(await this.#startLink(person));

        return address === undefined ? undefined : { space: address.space };
    }

    /** Report a failure no event answers to the person's home, else the host's standard error. */
    report(event: string, error: unknown): void {
        // capture it in the telemetry of the person signed in
        const person = this.#person;
        if (person !== undefined) {
            person.instruments.captureException(error, { isEscaped: true, tags: { event } });
        }
        // write it to the host while nobody is signed in
        else {
            this.environment.process.stderr.writeSync(`${event}: ${inspect(error)}\n`);
        }
    }

    /** Stop the person's renders and flush their telemetry, leaving the windows to restore. */
    async [Symbol.asyncDispose](): Promise<void> {
        // stop the sign-in and the person's renders
        this.#stopping.abort();
        await this.#signingIn?.done;
        await this.#person?.stop();

        // close the distribution's and the record's clients
        await this.#opened.disposeAsync();
    }

    /** Show whoever any program signs in on this computer, at every change until aborted. */
    async #followLogin(link: string | undefined, signal: AbortSignal): Promise<void> {
        // follow the signed-in people, the setting choosing among them and the machine's enrollment
        const { logins, settings } = this.native;
        const followed = RelationalQuery.latest(
            {
                signedIn: logins.query.login.findMany({}),
                chosen: settings.query.deviceSetting.findMany({ where: currentLogin.reference }),
                computers: this.computers.query.computer.findMany({}),
            },
            signal,
        );

        // show the person chosen at each change, reporting a failure and following on
        let pending = link;
        for await (const { signedIn, chosen, computers } of followed) {
            try {
                // open the launch link for the first person shown, or the start link without windows
                const [record] = computers;
                const isShown = await this.#replace(this.native.choose(signedIn, chosen), record);
                const windows = await this.objects.query.window.findMany();
                if (isShown && (pending !== undefined || windows.length === 0)) {
                    await this.openLink(pending ?? null);
                    pending = undefined;
                }
            } catch (error) {
                this.report("login.failed", error);
            }
        }
    }

    /** Offer the distribution's update commands in the menus as it changes. */
    async #followUpdates(signal: AbortSignal): Promise<void> {
        await this.menu.follow(
            "update",
            this.#distributions,
            this.#distributions.query.distribution.findFirst().watch(signal),
            [
                { command: checkForUpdates, after: (id) => this.#tellCurrent(id) },
                { command: restartToUpdate },
            ],
        );
    }

    /** Tell the person the installed release is the latest once a check staged none. */
    async #tellCurrent(id: string): Promise<void> {
        const checked = await this.#distributions.query.distribution.findFirst({
            where: { id: schema.identifier("distribution").parse(id) },
        });
        if (checked === undefined) {
            throw DesktopError.of("DISTRIBUTION_NOT_FOUND", `the distribution ${id} is gone`);
        } else if (checked.staged === null) {
            await this.notify(
                "current",
                this.localization.render(t`Destack is up to date`),
                this.localization.render(t`Destack ${argument("version")} is the latest release.`, {
                    version: checked.installed,
                }),
            );
        }
    }

    /** Replace the person shown with a login's once the machine enrolled, reporting a new one. */
    async #replace(
        login: Login["row"] | undefined,
        record: Select<typeof computer.table> | undefined,
    ): Promise<boolean> {
        // leave the person shown
        if (login?.id === this.#person?.session.login.row.id) {
            return false;
        }

        // stop the person shown before, leaving nothing of them on screen
        const before = this.#person;
        this.#person = undefined;
        if (before !== undefined) {
            await before.stop();
            await before.clear();
        }

        // offer signing in to nobody
        const enrollment = record?.enrollment;
        if (login === undefined) {
            await this.menu.set("account", Desktop.accountEntries({ status: "signedOut" }, this));

            return false;
        }
        // show the sign-in running until the person's sign-in enrolled the machine in their account
        else if (
            record === undefined ||
            enrollment?.kind !== "personal" ||
            enrollment.status !== "enrolled"
        ) {
            await this.menu.set("account", Desktop.accountEntries({ status: "signingIn" }, this));

            return false;
        }
        // stop a browser sign-in made stale by another program's, then show the login's person
        else {
            this.#signingIn?.stopping.abort();
            const session = await this.native.session(login.id);
            this.#person = await Person.open(this, session, record.machineId, enrollment.accountId);
            await this.menu.set(
                "account",
                Desktop.accountEntries({ status: "signedIn", login }, this),
            );

            return true;
        }
    }

    /** Read the person's start link: their setting, else Home's start view in their home. */
    async #startLink(person: Person): Promise<string> {
        // read the setting first
        const link = await this.native.setting(start);

        return link ?? this.links.format(await person.session.view("home"));
    }

    /** Read the person signed in, refusing while nobody is. */
    #requirePerson(): Person {
        if (this.#person === undefined) {
            throw DesktopError.of("SIGN_IN_REQUIRED", "nobody is signed in on this desktop");
        }

        return this.#person;
    }

    /** Change the window rows in the desktop's local objects. */
    #windows() {
        return this.objects.mutate(window);
    }
}

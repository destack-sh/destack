import type { Login } from "@destack/account/native";
import { inspect } from "node:util";
import { t } from "@destack/locale";
import { machine, placement, space } from "@destack/account/object";
import type { Device, Launcher } from "@destack/machine/device";
import type { Address } from "@destack/view/declare";
import type { ObjectClient } from "@destack/object/client";
import { setting } from "@destack/setting/object";
import { message } from "@destack/message/client";
import { activity, Inbox, type InboxRow, notification } from "@destack/notification/client";
import { OBSERVABILITY_PACKAGE } from "@destack/observability/client";
import type {} from "@destack/package/import-meta";
import { type Identifier, present, schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import type { Telemetry } from "@destack/telemetry/sdk";
import { type DESKTOP_OBJECTS, window, type WindowRow } from "../object/index.ts";
import { Link } from "../link/index.ts";
import {
    Badge,
    Banner,
    BannerCapabilities,
    Shortcut,
    Tray,
    Window,
    type WindowContext,
    type WindowState,
    type WindowStep,
} from "../server/index.ts";
import type { Event } from "../shell/index.ts";
import { DesktopError } from "../error/index.ts";
import type { Desktop } from "./desktop.ts";

/** The origin the telemetry exporter addresses, which the launcher routes to the home. */
const TELEMETRY_ORIGIN = "http://observability.localhost";

/** The shortcut floating the command palette, as the person's `shortcut` setting binds it. */
export const PALETTE_SHORTCUT = "palette";

/** The events a person answers: all but menu choices, links, reopening and failures. */
export type PersonEvent = Exclude<
    Event,
    {
        readonly action: "menu.chosen" | "shell.failed" | "link.opened" | "application.reopened";
    }
>;

/** What a person uses of the desktop: its objects, shell, menus and language. */
export type PersonDesktop = Pick<
    Desktop,
    | "objects"
    | "computers"
    | "shell"
    | "menu"
    | "links"
    | "localization"
    | "report"
    | "openPalette"
    | "openSettings"
    | "notify"
> & {
    /** This computer's device: its issuer, views it launches and visits of objects it opens. */
    readonly native: Pick<Device, "launch" | "open"> & {
        /** The issuer people sign in at, whose origin signs the windows' origins in. */
        readonly issuer: Pick<Device["issuer"], "url">;
    };
};

/** What a person uses of their launcher: their calls and views. */
export type PersonLauncher = Pick<Launcher, "user" | "url" | "address" | "follow" | "view">;

/** The person signed in on this computer, and the windows and icons showing what is theirs. */
export class Person {
    /** The person's launcher on this computer. */
    readonly launcher: PersonLauncher;
    /** The person's sign-in on this computer: its client, name and keychain entry. */
    readonly login: Login;
    /** The banners the shell shows, by message. */
    readonly banners: Map<string, Banner>;
    /** The application icon's unread count. */
    readonly badge: Badge;
    /** The tray's status and the spaces this computer serves. */
    readonly tray: Tray;
    /** The shortcut floating the command palette. */
    readonly shortcut: Shortcut;
    /** The person's inbox replicated from their home, which the banners and the badge follow. */
    readonly inbox: ObjectClient<{
        readonly notification: typeof notification;
        readonly activity: typeof activity;
        readonly message: typeof message;
    }>;
    /** This computer's machine and the account's spaces, replicated for the tray. */
    readonly account: ObjectClient<{
        readonly machine: typeof machine;
        readonly placement: typeof placement;
        readonly space: typeof space;
    }>;
    /** The person's settings replicated from the universe, which the shortcut follows. */
    readonly settings: ObjectClient<{ readonly setting: typeof setting }>;
    /** The desktop's telemetry while the person is signed in, exported to their home. */
    readonly telemetry: Telemetry;
    /** The desktop's failures, captured in that telemetry. */
    readonly instruments: TelemetryScope;
    /** The desktop the person shows on. */
    readonly #desktop: PersonDesktop;
    /** The rows, shell and failures the windows work with. */
    readonly #context: WindowContext;
    /** Where each row's window stands, by row: launching or shown. */
    readonly #windows: Map<string, WindowState>;
    /** The client the person's sign-in registered, which desktop messages address. */
    readonly #client: Identifier<"client">;
    /** What the system's banners offer here. */
    readonly #capabilities: BannerCapabilities;
    /** Aborts once the person signs out or the desktop stops. */
    readonly #stopping: AbortController;
    /** The window last in focus, absent before any. */
    #focused: string | undefined;
    /** The clients and the telemetry the person opened, closed once they stop. */
    readonly #opened: AsyncDisposableStack;
    /** The follows of the badge, tray, shortcut, windows and banners, running until stopped. */
    #running: Promise<unknown>;

    /** Show a person on a desktop from their launcher, clients and telemetry. */
    constructor(
        desktop: PersonDesktop,
        fields: Pick<
            Person,
            "launcher" | "login" | "inbox" | "account" | "settings" | "telemetry"
        > & {
            readonly machineId: Identifier<"machine">;
            readonly capabilities: BannerCapabilities;
        },
    ) {
        // set the launcher, the clients and the telemetry
        this.launcher = fields.launcher;
        this.login = fields.login;
        this.inbox = fields.inbox;
        this.account = fields.account;
        this.settings = fields.settings;
        this.telemetry = fields.telemetry;
        this.instruments = TelemetryScope.of(import.meta.destack.package, fields.telemetry);
        this.#desktop = desktop;

        // show nothing yet
        const { shell, menu, objects } = desktop;
        this.#windows = new Map<string, WindowState>();
        this.banners = new Map<string, Banner>();
        this.badge = new Badge(shell);
        this.tray = new Tray({
            account: fields.account,
            computers: desktop.computers,
            launcher: fields.launcher,
            machineId: fields.machineId,
            menu,
            shell,
            localization: desktop.localization,
            open: (address) => desktop.openPalette({ space: address }),
        });
        this.shortcut = new Shortcut(PALETTE_SHORTCUT, shell);
        this.#context = {
            objects,
            shell,
            report: (error) => desktop.report("window.failed", error),
            universe: new URL(desktop.native.issuer.url).origin,
        };
        this.#client = schema.identifier("client").parse(fields.login.row.clientId);
        this.#capabilities = fields.capabilities;
        this.#stopping = new AbortController();
        this.#opened = new AsyncDisposableStack();
        this.#running = Promise.resolve();
    }

    /** Open what a signed-in person sees on this machine of their account, and follow it. */
    static async open(
        desktop: PersonDesktop & Pick<Desktop, "native" | "environment">,
        launcher: Launcher,
        login: Login,
        machineId: Identifier<"machine">,
        accountId: Identifier<"account">,
    ): Promise<Person> {
        // close what opened before a failure
        await using opened = new AsyncDisposableStack();
        const home = { space: await launcher.home() };

        // export the desktop's telemetry to the observability of the person's home
        const exporter = OtlpExporter.http(
            {
                endpoint: TELEMETRY_ORIGIN,
                fetch: async (request) =>
                    launcher.send(
                        request,
                        OBSERVABILITY_PACKAGE.id,
                        await desktop.native.locate(home.space),
                    ),
            },
            (error) =>
                desktop.environment.process.stderr.writeSync(`telemetry: ${inspect(error)}\n`),
        );
        const telemetry = await startTelemetry(
            exporter.options(import.meta.destack.package),
            desktop.environment,
        );
        opened.defer(() => telemetry.shutdown());

        // replicate the person's inbox, the account's spaces and their settings
        const inbox = await desktop.native.open(
            launcher,
            home,
            { notification, activity, message },
            opened,
        );
        const account = await desktop.native.open(
            launcher,
            { scope: accountId },
            { machine, placement, space },
            opened,
        );
        const settings = await desktop.native.open(
            launcher,
            { scope: launcher.user.subject.id },
            { setting },
            opened,
        );

        // show the person, who closes what opened once they leave
        const person = new Person(desktop, {
            launcher,
            login,
            inbox,
            account,
            settings,
            telemetry,
            machineId,
            capabilities: BannerCapabilities.of(desktop.environment.host.os),
        });
        await person.closePalettes();
        await person.#start();
        person.#opened.use(opened.move());

        return person;
    }

    /** Answer one of the shell's events, dropping one about a window or banner gone already. */
    async answer(event: PersonEvent): Promise<void> {
        // refuse a shortcut nothing registered
        if (event.action === "shortcut.pressed" && event.id !== this.shortcut.id) {
            throw DesktopError.of("SHORTCUT_NOT_FOUND", `shortcut '${event.id}' not found`);
        }
        // float the palette on its shortcut
        else if (event.action === "shortcut.pressed") {
            await this.#desktop.openPalette();
        }
        // close the row of a window the person closed
        else if (event.action === "window.closed") {
            await this.closed(event.id);
        }
        // write where the person placed a window once the drag rests
        else if (event.action === "window.moved") {
            this.#shown(event.id)?.place(event.bounds);
        }
        // open a location a window navigated to off its origin
        else if (event.action === "window.navigated") {
            await this.navigated(event.id, event.url);
        }
        // remember the window in focus while it shows
        else if (event.action === "window.focused") {
            this.#focused = this.#shown(event.id) === undefined ? this.#focused : event.id;
        }
        // answer a banner's action as the person
        else if (event.action === "banner.acted") {
            await this.banners
                .get(event.id)
                ?.act(this.launcher, this.#desktop.native, event.name, event.text);
        }
        // leave a dismissed banner's notification unread
        else {
            event.action satisfies "banner.dismissed";
        }
    }

    /** Make the windows match the rows, leaving the ones launching. */
    async showWindows(rows: readonly WindowRow[]): Promise<void> {
        // plan the steps, then apply them together
        const steps = Window.plan(this.#windows, rows);
        await Promise.all(steps.map((step) => this.#apply(step)));
    }

    /** Close every palette, as at most one floats and only while it has focus. */
    async closePalettes(): Promise<void> {
        // close each open palette's row
        const floating = await this.#desktop.objects.query.window.findMany({
            where: { kind: "palette", deletedAt: { isNull: true } },
        });
        await Promise.all(floating.map((row) => this.#rows().close({ id: row.id }).confirmed));
    }

    /** Close the row of a window the shell closed, which the next plan removes. */
    async closed(id: Identifier<"window">): Promise<void> {
        // drop a close of a window this desktop removed already
        const shown = this.#shown(id);
        if (shown === undefined) {
            return;
        }

        // stop the window's place waiting out a drag, then close its row
        shown.stop();
        this.#windows.delete(id);
        await this.#rows().close({ id }).confirmed;
    }

    /** Open a location a window navigated to off its origin. */
    async navigated(id: Identifier<"window">, href: string): Promise<void> {
        // hand a location outside the views beside the window to the browser
        const source = this.#shown(id);
        const address = source?.beside(href);
        if (source === undefined || address === undefined) {
            await this.#desktop.shell.send({ action: "browser.open", url: href });
        }
        // open the view in a new window, closing the palette or an object's window that led to it
        else {
            await this.#rows().open({ link: this.#desktop.links.format(address) }).confirmed;
            if (source.kind === "palette" || "object" in source.address) {
                await this.closed(id);
                await source.close();
            }
        }
    }

    /** Read the window last in focus while it shows. */
    inFocus(): Window | undefined {
        return this.#focused === undefined ? undefined : this.#shown(this.#focused);
    }

    /** Read the window the person returns to: the one last in focus, else the first shown. */
    front(): Window | undefined {
        // take the window in focus, else the first shown
        const shown = [...this.#windows.values()].flatMap((state) =>
            state.kind === "shown" ? [state.window] : [],
        );

        return this.inFocus() ?? shown[0];
    }

    /** Show the new banners of this client's unread messages, and record them received. */
    async showBanners(
        messages: Parameters<typeof Banner.select>[0],
        rows: readonly InboxRow[],
    ): Promise<void> {
        // select the banners of the client's unread messages, and plan the shell's commands
        const now = this.inbox.environment.clock.now();
        const selected = Banner.select(messages, new Inbox(rows, now), this.#client);
        const commands = Banner.plan(this.banners, selected, this.#capabilities);
        for (const command of commands) {
            await this.#desktop.shell.send(command);
        }

        // select the banners to show, and record the new ones the replica pushes as received
        this.banners.clear();
        for (const banner of selected) {
            this.banners.set(banner.id, banner);
        }
        const shown = commands.flatMap((command) =>
            command.action === "banner.show" ? [command.id] : [],
        );
        const receipts = this.inbox.mutate(message);
        await Promise.all(shown.map((id) => receipts.receive({ id }).confirmed));
    }

    /** Stop the follows and the windows' places, then close the clients and flush. */
    async stop(): Promise<void> {
        // stop the follows and every place waiting out a drag
        this.#stopping.abort();
        for (const state of this.#windows.values()) {
            if (state.kind === "shown") {
                state.window.stop();
            }
        }
        await this.#running;

        // close the clients, then flush the telemetry
        await this.#opened.disposeAsync();
    }

    /** Remove what the person sees: windows, banners, badge, tray, shortcut and menus. */
    async clear(): Promise<void> {
        // close the shown windows, then close and remove every row
        for (const state of this.#windows.values()) {
            if (state.kind === "shown") {
                await state.window.close();
            }
        }
        this.#windows.clear();
        for (const row of await this.#desktop.objects.query.window.findMany()) {
            if (row.deletedAt === null) {
                await this.#rows().close({ id: row.id }).confirmed;
            }
            await this.#rows().finalize({ id: row.id }).confirmed;
        }

        // withdraw the banners, then empty the badge, the tray, the shortcut and the settings item
        for (const command of Banner.plan(this.banners, [], this.#capabilities)) {
            await this.#desktop.shell.send(command);
        }
        this.banners.clear();
        await this.badge.clear();
        await this.tray.clear();
        await this.shortcut.clear();
        await this.#desktop.menu.set("settings", []);
    }

    /** Follow the person's rows until stopped: windows, banners, badge, tray and shortcut. */
    async #start(): Promise<void> {
        // offer the settings in the menus
        const { menu, shell, localization, objects } = this.#desktop;
        const report = (event: string, error: unknown) => this.#desktop.report(event, error);
        await menu.set("settings", [
            {
                kind: "action",
                id: "open",
                title: localization.render(t`Settings…`),
                shortcut: "mod+,",
                run: () => this.#desktop.openSettings(),
            },
        ]);

        // follow each, quitting once the windows fail
        const { signal } = this.#stopping;
        const { inbox } = this;
        this.#running = Promise.all([
            this.#followWindows(objects, signal).catch(async (error: unknown) => {
                report("windows.failed", error);
                await shell.send({ action: "application.quit" });
            }),
            this.#followBanners(signal).catch((error: unknown) => report("inbox.failed", error)),
            this.badge
                .follow(inbox, signal)
                .catch((error: unknown) => report("inbox.failed", error)),
            this.tray.follow(signal).catch(async (error: unknown) => {
                report("serving.failed", error);
                await this.tray.failed();
            }),
            this.shortcut
                .follow(this.settings, this.login.row.subjectId, signal)
                .catch((error: unknown) => report("shortcut.failed", error)),
        ]);
    }

    /** Show the windows at each change of their rows, until stopped. */
    async #followWindows(
        objects: ObjectClient<typeof DESKTOP_OBJECTS>,
        signal: AbortSignal,
    ): Promise<void> {
        for await (const rows of objects.query.window.findMany().watch(signal)) {
            await this.showWindows(rows);
        }
    }

    /** Show the banners at each change of the inbox and its messages, until stopped. */
    async #followBanners(signal: AbortSignal): Promise<void> {
        for await (const { messages, notifications } of Banner.follow(this.inbox, signal)) {
            await this.showBanners(messages, notifications);
        }
    }

    /** Apply one step of a plan of the windows. */
    async #apply(step: WindowStep): Promise<void> {
        // launch and show a new row
        if (step.kind === "show") {
            await this.#launch(step.row);
        }
        // focus a shown window
        else if (step.kind === "focus") {
            await this.#requireShown(step.row.id).focus(step.row);
        }
        // observe a change the shell already shows
        else if (step.kind === "settle") {
            await this.#requireShown(step.row.id).observe(step.row);
        }
        // close a duplicate row and focus the window showing its link
        else if (step.kind === "merge") {
            await this.#rows().close({ id: step.id }).confirmed;
            await this.#rows().focus({ id: step.into }).confirmed;
        }
        // close a removed row's window, and remove a closing row
        else {
            const shown = this.#shown(step.id);
            this.#windows.delete(step.id);
            await shown?.close();
            if (step.isClosing) {
                await this.#rows().finalize({ id: step.id }).confirmed;
            }
        }
    }

    /** Launch a row's link and show it, refusing a link that fails. */
    async #launch(row: WindowRow): Promise<void> {
        // refuse a link to no view or object
        const { links, localization } = this.#desktop;
        const address = links.parse(row.link);
        if (address === undefined) {
            await this.#refuse(row.id, `${row.link} addresses no Destack view or object`);
        }
        // show it under its space's or installation's name, the palette under its link's title
        else {
            const title =
                row.kind === "palette" ? localization.render(t`Commands`) : Link.title(address);
            this.#windows.set(row.id, { kind: "launching" });
            try {
                const launching = {
                    launch: (asked: Address) => this.#desktop.native.launch(this.launcher, asked),
                };
                const opened = await Window.open(row, address, title, launching, this.#context);
                await this.#record(row.id, opened);
            } catch (error) {
                // tell the person why the window failed and close its row
                const reason = error instanceof Error ? error.message : String(error);
                await this.#refuse(row.id, reason);
            }
        }
    }

    /** Record a launched window shown, closing it at once when its row closed while it launched. */
    async #record(id: string, opened: Window): Promise<void> {
        // close a window whose row the plan removed while it launched
        if (this.#windows.get(id)?.kind !== "launching") {
            await opened.close();
        }
        // show it otherwise
        else {
            this.#windows.set(id, { kind: "shown", window: opened });
        }
    }

    /** Tell the person why a window cannot open, and close its row. */
    async #refuse(id: string, reason: string): Promise<void> {
        this.#windows.delete(id);
        await this.#desktop.notify(
            id,
            this.#desktop.localization.render(t`Cannot open link`),
            reason,
        );
        await this.#rows().close({ id }).confirmed;
    }

    /** Read a shown window, absent for a row not shown here. */
    #shown(id: string): Window | undefined {
        const state = this.#windows.get(id);

        return state?.kind === "shown" ? state.window : undefined;
    }

    /** Read a shown window, which a planned step requires. */
    #requireShown(id: string): Window {
        return present(this.#shown(id), `the shown window ${id}`);
    }

    /** Change the window rows in the desktop's local objects. */
    #rows() {
        return this.#desktop.objects.mutate(window);
    }
}

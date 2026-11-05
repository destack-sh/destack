import { machine, placement, space } from "@destack/account/object";
import { Login, login, SessionStore } from "@destack/account/native";
import { MemoryKeychain } from "@destack/identity";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { MachineError } from "@destack/machine/error";
import { computer } from "@destack/machine/device";
import { message } from "@destack/message";
import { activity, notification } from "@destack/notification";
import { Bookmark } from "@destack/object/client";
import { testSimulation } from "@destack/object/test";
import { PackageId } from "@destack/package";
import { Identifier, present } from "@destack/schema";
import { setting } from "@destack/setting/object";
import { startSilentTelemetry } from "@destack/telemetry/test";
import { expect, test } from "@destack/test";
import type { Address } from "@destack/view/declare";
import { Link } from "../link/index.ts";
import { window } from "../object/index.ts";
import { BannerCapabilities, Menu } from "../server/index.ts";
import { desktopObjects, localObjects, recordShell } from "../test/index.ts";
import { Person, type PersonSession } from "./person.ts";

/** The links of the stable desktop. */
const links = new Link("destack");

/** The view the windows open. */
const NOTES = "destack://notes--personal--florian/home";

/** The board the windows open. */
const TASKS = "destack://tasks--personal--florian/board";

/** The package whose settings the views ask to read. */
const SETTINGS = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

/** Read the installation a view address opens. */
function name(address: Address): string {
    return "installation" in address ? address.installation : "space";
}

/** Describe the dialog asking the person to grant a view reading their settings. */
function dialog(id: string, view: string) {
    return {
        action: "dialog.ask",
        id,
        title: `Allow ⁨${view}⁩ to act for you?`,
        body: "read setting",
        allow: "Allow",
        deny: "Don’t Allow",
    };
}

/** Refuse a call the person makes to a space, which these tests never make. */
function refuse(): Promise<never> {
    return Promise.reject(new TypeError("the person calls no space here"));
}

/** Show a person launching and granting views as given, on a separate desktop. */
async function signedIn(
    simulation: ReturnType<typeof testSimulation>,
    session: Pick<PersonSession, "launch" | "grant">,
) {
    // store the windows and the person's login in memory, and record what the shell is sent
    const environment = simulation.environment("desktop");
    const objects = await desktopObjects(environment);
    const { shell, sent } = recordShell();
    const localization = Localization.of(SOURCE_LOCALE, []);
    const logins = await localObjects({ login }, environment);
    const created = logins.mutate(login).create({
        issuer: "https://destack.test",
        subjectId: Identifier.create("user", environment),
        clientId: Identifier.create("client", environment),
        name: "Florian",
        status: "authenticated",
    });
    await created.confirmed;
    const row = present(
        await logins.query.login.findFirst({ where: { id: (await created.predicted).id } }),
        "the login",
    );

    // record this computer's machine with an open tunnel, replicating nothing from the person's scopes
    const machineId = Identifier.create("machine", environment);
    const computers = await localObjects({ computer }, environment);
    await computers.call(computer, "create", {
        ...computer.in(computers.scope),
        machineId,
        version: "2026.11.0",
        pid: 1,
        startedAt: environment.clock.now(),
        tunnel: "open",
    }).predicted;

    // show the person on a separate desktop
    const person = new Person(
        {
            objects,
            computers,
            shell,
            menu: new Menu(shell, localization),
            links,
            localization,
            report: (_, error) => {
                throw error;
            },
            openPalette: async () => undefined,
            openSettings: async () => undefined,
            notify: (key, title, body) =>
                shell.send({
                    action: "banner.show",
                    id: `notice.${key}`,
                    title,
                    body,
                    interruption: "active",
                    actions: [],
                }),
        },
        {
            session: {
                ...session,
                login: new Login(
                    row,
                    new SessionStore(new MemoryKeychain()),
                    refuse,
                    environment,
                    new Bookmark(),
                ),
                address: refuse,
                open: refuse,
                view: refuse,
            },
            inbox: await localObjects({ notification, activity, message }, environment),
            account: await localObjects({ machine, placement, space }, environment),
            settings: await localObjects({ setting }, environment),
            telemetry: await startSilentTelemetry(
                {
                    name: "desktop",
                    version: "0.0.0",
                    metrics: {},
                    signals: () => ({ traces: {}, logs: {} }),
                    report: (error) => {
                        throw error;
                    },
                },
                environment,
            ),
            machineId,
            capabilities: BannerCapabilities.of("darwin"),
        },
    );

    return { person, objects, sent };
}

test("return to the window last in focus, and close every window and row on clear", async () => {
    // show two views as a person launching each on its origin
    const simulation = testSimulation();
    const { person, objects, sent } = await signedIn(simulation, {
        launch: async (address) => ({
            url: `http://${name(address)}--personal--florian.localhost:7470/`,
            target: Identifier.create("installation", objects.environment),
        }),
        grant: () => Promise.reject(new TypeError("no view asks for a grant here")),
    });
    const notes = await objects.mutate(window).open({ link: NOTES }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const board = await objects.mutate(window).open({ link: TASKS }).predicted;
    await person.showWindows(await objects.query.window.findMany());

    // return to the first shown, then to the one focused last
    const first = person.front()?.id;
    await person.answer({ action: "window.focused", id: board.id });
    const focused = person.front()?.id;

    // clear what the person sees
    sent.length = 0;
    await person.clear();

    // expect the windows closed on screen, every menu and icon emptied, and the rows gone
    expect({
        first,
        focused,
        cleared: sent,
        rows: await objects.query.window.findMany(),
        front: person.front(),
    }).toEqual({
        first: notes.id,
        focused: board.id,
        cleared: [
            { action: "window.close", id: notes.id },
            { action: "window.close", id: board.id },
            { action: "badge.set", count: null },
            { action: "menu.set", menu: "tray", items: [] },
            { action: "tray.set", tooltip: "Destack" },
            { action: "shortcut.unregister", id: "palette" },
            { action: "menu.set", menu: "application", items: [] },
        ],
        rows: [],
        front: undefined,
    });
});

test("ask the person in a dialog to grant a view, showing it once allowed, closing it once refused, and close a link failing to open", async () => {
    // launch a view only once the person granted its installation what it asks
    const simulation = testSimulation();
    const granted = new Set<string>();
    const targets: Identifier<"installation">[] = [];
    const { person, objects, sent } = await signedIn(simulation, {
        launch: async (address) => {
            if (!granted.has(name(address))) {
                throw MachineError.of("GRANT_REQUIRED", `${name(address)} asks for a grant`, {
                    data: {
                        permissions: [
                            { packageId: SETTINGS, type: "setting", name: "read", scope: "user-1" },
                            {
                                packageId: SETTINGS,
                                type: "setting",
                                name: "read",
                                scope: "space-1",
                            },
                        ],
                    },
                });
            }

            const target = Identifier.create("installation", objects.environment);
            targets.push(target);

            return { url: `http://${name(address)}--personal--florian.localhost:7470/`, target };
        },
        grant: async (address) => {
            granted.add(name(address));

            return { id: Identifier.create("grant", objects.environment), scope: "user-1" };
        },
    });

    // open notes, the board and a link addressing nothing, then plan again while the dialogs wait
    const windows = objects.mutate(window);
    const notes = await windows.open({ link: NOTES }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const board = await windows.open({ link: TASKS }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const nothing = await windows.open({ link: "destack://nothing" }).predicted;
    await person.showWindows(await objects.query.window.findMany());
    await person.showWindows(await objects.query.window.findMany());
    const asked = sent.splice(0);

    // allow notes and refuse the board
    await person.answer({ action: "dialog.answered", id: notes.id, isAllowed: true });
    await person.answer({ action: "dialog.answered", id: board.id, isAllowed: false });
    const rows = await objects.query.window.findMany();

    // expect the failed link removed, one dialog per view, notes shown and the board closed
    expect({
        asked,
        answered: sent,
        closed: rows.map((row) => ({ id: row.id, isClosed: row.deletedAt !== null })),
    }).toEqual({
        asked: [
            {
                action: "banner.show",
                id: `notice.${nothing.id}`,
                title: "Cannot open link",
                body: "destack://nothing addresses no Destack view or object",
                interruption: "active",
                actions: [],
            },
            dialog(notes.id, "notes"),
            dialog(board.id, "tasks"),
        ],
        answered: [
            {
                action: "window.open",
                id: notes.id,
                kind: "view",
                url: "http://notes--personal--florian.localhost:7470/",
                profile: Identifier.uuid(present(targets[0], "the launched installation")),
                title: "notes",
            },
        ],
        closed: [
            { id: notes.id, isClosed: false },
            { id: board.id, isClosed: true },
        ],
    });
});

test("open a view a window navigated to beside it in a new window, and hand any other location to the browser", async () => {
    // show notes as a person launching each view on its origin
    const simulation = testSimulation();
    const { person, objects, sent } = await signedIn(simulation, {
        launch: async (address) => ({
            url: `http://${name(address)}--personal--florian.localhost:7470/home`,
            target: Identifier.create("installation", objects.environment),
        }),
        grant: () => Promise.reject(new TypeError("no view asks for a grant here")),
    });
    const notes = await objects.mutate(window).open({ link: NOTES }).predicted;
    await person.showWindows(await objects.query.window.findMany());
    sent.length = 0;

    // navigate to the board beside notes, then to a site off the views
    await person.navigated(notes.id, "http://tasks--personal--florian.localhost:7470/board");
    await person.navigated(notes.id, "https://example.test/docs");
    const rows = await objects.query.window.findMany();

    // expect the board opened as a second row beside notes, and the site in the browser
    expect({ links: rows.map((row) => row.link), sent }).toEqual({
        links: [NOTES, TASKS],
        sent: [{ action: "browser.open", url: "https://example.test/docs" }],
    });
});

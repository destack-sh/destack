import { machine, placement, space } from "@destack/account/object";
import { Login, login } from "@destack/account/native";
import { TokenCache } from "@destack/access/token";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { computer } from "@destack/machine/device";
import { message } from "@destack/message/object";
import { activity, notification } from "@destack/notification/object";
import { Bookmark } from "@destack/object/client";
import { testSimulation } from "@destack/object/test";
import { Identifier, present, schema } from "@destack/schema";
import { setting } from "@destack/setting/object";
import { startSilentTelemetry } from "@destack/telemetry/test";
import { expect, test } from "@destack/test";
import type { Address } from "@destack/view/declare";
import { Link } from "../link/index.ts";
import { window } from "../object/index.ts";
import { BannerCapabilities, Menu, PERSON_PROFILE } from "../server/index.ts";
import { desktopObjects, localObjects, recordShell } from "../test/index.ts";
import { Person, type PersonLauncher } from "./person.ts";

/** The links of the stable desktop. */
const links = new Link("destack");

/** The view the windows open. */
const NOTES = "destack://notes--personal--florian/home";

/** The board the windows open. */
const TASKS = "destack://tasks--personal--florian/board";

/** Read the installation a view address opens. */
function name(address: Address): string {
    return "installation" in address ? address.installation : "space";
}

/** The space the launched views run in. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Refuse a call the person makes to a space, which these tests never make. */
function refuse(): Promise<never> {
    return Promise.reject(new TypeError("the person calls no space here"));
}

/** Show a person launching and granting views as given, on a separate desktop. */
async function signedIn(
    simulation: ReturnType<typeof testSimulation>,
    launcher: Pick<PersonLauncher, "url">,
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
    const signedInLogin = new Login(
        row,
        { tokens: () => new TokenCache(), issue: refuse, prove: refuse, forget: refuse },
        refuse,
        environment,
        new Bookmark(),
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
            native: {
                launch: (launching, address) => launching.url(address),
                open: refuse,
                issuer: { url: "https://destack.test/oauth" },
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
            launcher: {
                ...launcher,
                user: signedInLogin.user,
                address: refuse,
                follow: refuse,
                view: refuse,
            },
            login: signedInLogin,
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
        url: async (address) => ({
            url: `http://${name(address)}--personal--florian.localhost:7470/`,
            space: SPACE,
        }),
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

test("open each view on its origin in the person's profile, signed in through the universe, and close a link failing to open", async () => {
    // address each view's page on its origin
    const simulation = testSimulation();
    const { person, objects, sent } = await signedIn(simulation, {
        url: async (address) => ({
            url: `http://${name(address)}--personal--florian.localhost:7470/`,
            space: SPACE,
        }),
    });

    // open notes and a link addressing nothing
    const windows = objects.mutate(window);
    const notes = await windows.open({ link: NOTES }).predicted;
    await simulation.jump(simulation.clock.now() + 1);
    const nothing = await windows.open({ link: "destack://nothing" }).predicted;
    await person.showWindows(await objects.query.window.findMany());
    const rows = await objects.query.window.findMany();

    // expect notes shown in the person's profile within the universe, and the failed link closed
    expect({
        sent,
        closed: rows.map((row) => ({ id: row.id, isClosed: row.deletedAt !== null })),
    }).toEqual({
        sent: [
            {
                action: "banner.show",
                id: `notice.${nothing.id}`,
                title: "Cannot open link",
                body: "destack://nothing addresses no Destack view or object",
                interruption: "active",
                actions: [],
            },
            {
                action: "window.open",
                id: notes.id,
                kind: "view",
                url: "http://notes--personal--florian.localhost:7470/",
                origins: ["https://destack.test"],
                profile: PERSON_PROFILE,
                title: "notes",
            },
        ],
        closed: [
            { id: notes.id, isClosed: false },
            { id: nothing.id, isClosed: true },
        ],
    });
});

test("open a view a window navigated to beside it in a new window, and hand any other location to the browser", async () => {
    // show notes as a person launching each view on its origin
    const simulation = testSimulation();
    const { person, objects, sent } = await signedIn(simulation, {
        url: async (address) => ({
            url: `http://${name(address)}--personal--florian.localhost:7470/home`,
            space: SPACE,
        }),
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

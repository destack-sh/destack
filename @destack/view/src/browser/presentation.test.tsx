import type { Simulation } from "@destack/environment/simulation";
import { testSimulation } from "@destack/test/world";
import { TestDatabase } from "@destack/db/test";
import { PackageVersion } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { Digest, type JsonValue, schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { deviceSetting, setting } from "@destack/setting/object";
import { DEFAULT_DISPLAY, Theme, TOKENS } from "@destack/theme";
import { defineTheme, destackTheme } from "@destack/theme/declare";
import { ANONYMOUS, principal, accessRelationship } from "@destack/access";
import { expect, onTestFinished, test } from "@destack/test";
import { keybindings, sources } from "../palette/index.ts";
import { appearance, theme } from "../setting/index.ts";
import { SimulatedStorage } from "@destack/environment/browser/simulation";
import { Presentation, type Page } from "./presentation.ts";

/** The person a page presents for. */
const PERSON = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000e2");

/** The notes package whose view the page shows. */
const notes = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000e1",
    name: "@example/notes",
    version: "2026.9.0",
});

/** A page's ground. */
const GROUND = {
    light: { background: "#fef2f4", foreground: "#42222f", muted: "#f6e6ea" },
    dark: { background: "#200f16", foreground: "#e8d6dc", muted: "#2e1c23" },
};

/** A person's ground. */
const CHOSEN = {
    light: { background: "#f1f9ef", foreground: "#1e3323", muted: "#e2ece1" },
    dark: { background: "#0c1b10", foreground: "#d8e3d6", muted: "#1c2c20" },
};

/** The notes home view's page. */
const page: Page = {
    context: {
        installation: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000e3"),
        space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e4"),
        account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-0000000000e5"),
        view: "home",
        build: Digest.parse("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        base: "",
        universe: "https://destack.app",
        catalogs: [],
        spaceAddress: "home--example",
        spaceTitle: "Home",
        apps: [],
        user: principal.user.reference("universe", PERSON),
        home: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e6"),
    },
    packageId: notes.id,
    theme: destackTheme,
};

/** Open a database of settings, placing the person's values in it. */
async function placing(environment: ReturnType<Simulation["environment"]>) {
    // open the database, closed with the test
    const storage = await TestDatabase.create(
        [accessRelationship, setting.table, deviceSetting.table],
        { environment, isMigrated: true },
    );
    onTestFinished(() => storage.close());

    // place each value under its own id
    let placed = 0;
    const place = async (declared: Pick<Setting, "reference" | "package">, value: JsonValue) => {
        const now = environment.clock.now();
        placed += 1;
        await storage.database.transaction(
            async (transaction) =>
                await transaction.insert(setting.table).values({
                    id: schema
                        .identifier("setting")
                        .parse(`setting-01996ab0-0000-7000-8000-0000000000e${placed}`),
                    scope: PERSON,
                    packageId: declared.reference.packageId,
                    name: declared.reference.name,
                    mode: "set",
                    value,
                    release: declared.package.version,
                    edition: 1,
                    createdAt: now,
                    updatedAt: now,
                }),
        );
    };

    return { storage, place };
}

test("present a page in its person's language, their display settings' style and their keybindings, offering no commands once they turn the package off, and its device's own values over theirs", async () => {
    // place the person's dark appearance and their keybinding of the archive command, each under its own id
    await using simulation = testSimulation();
    const environment = simulation.environment("page");
    const { storage, place } = await placing(environment);
    const archive = DeclarationReference.key({ packageId: notes.id, name: "archive" });
    await place(appearance, "dark");
    await place(keybindings, { [archive]: "mod+a" });

    // present the page, then again once the person turns off the notes package's commands
    const person = { locale: "de-AT", chain: [PERSON] };
    const values = () => storage.database.select().from(setting.table);
    const presented = Presentation.resolve(page, person, await values(), []);
    await place(sources, { [notes.id]: false });
    const turnedOff = Presentation.resolve(page, person, await values(), []);

    // present it on a device keeping the light appearance for itself, over the person's dark one
    const now = environment.clock.now();
    await storage.database.transaction(
        async (transaction) =>
            await transaction.insert(deviceSetting.table).values({
                id: schema
                    .identifier("device-setting")
                    .parse("device-setting-01996ab0-0000-7000-8000-0000000000e8"),
                scope: schema
                    .identifier("device")
                    .parse("device-01996ab0-0000-7000-8000-0000000000e9"),
                packageId: appearance.reference.packageId,
                name: appearance.reference.name,
                value: "light",
                release: appearance.package.version,
                edition: 1,
                createdAt: now,
                updatedAt: now,
            }),
    );
    const kept = await storage.database.select().from(deviceSetting.table);
    const onDevice = Presentation.resolve(page, person, await values(), kept);

    // expect the person's presentation with their keybindings, commands off once turned off, and the device's style
    expect({
        presented,
        turnedOff: turnedOff.isOffered,
        onDevice: onDevice.style,
    }).toEqual({
        presented: {
            locale: "de-AT",
            style: destackTheme.variables({ ...DEFAULT_DISPLAY, appearance: "dark" }),
            keybindings: { [archive]: "mod+a" },
            isOffered: true,
        },
        turnedOff: false,
        onDevice: destackTheme.variables({ ...DEFAULT_DISPLAY, appearance: "light" }),
    });
});

test("present a page to an anonymous visitor in the browser's language and the theme's defaults, without keybindings of their own", () => {
    // present the notes page to a visitor, who keeps no settings anywhere
    const { home: _home, ...context } = page.context;
    const visited: Page = { ...page, context: { ...context, user: ANONYMOUS } };

    expect(Presentation.resolve(visited, { locale: "fr-FR", chain: [] }, [], [])).toEqual({
        locale: "fr-FR",
        style: destackTheme.variables(DEFAULT_DISPLAY),
        keybindings: {},
        isOffered: true,
    });
});

test("show a person the theme they chose, else Destack's, and a visitor the page's", async () => {
    // give the page a theme of its own, and place the person's choice of another
    await using simulation = testSimulation();
    const { storage, place } = await placing(simulation.environment("page"));
    const own = defineTheme({ name: "own", ground: GROUND }, { package: notes });
    const chosen = { name: "chosen", ground: CHOSEN };
    const unchosen = await storage.database.select().from(setting.table);
    await place(theme, chosen);
    const values = await storage.database.select().from(setting.table);

    // read the page's background for its person before and after they choose, and for a visitor
    const person = { locale: "en", chain: [PERSON] } as const;
    const shown = { ...page, theme: own };
    const visiting = { ...shown, context: { ...page.context, user: ANONYMOUS } };
    const background = TOKENS.entry(["color", "background"]).variable;
    const presented = [
        Presentation.resolve(shown, person, unchosen, []),
        Presentation.resolve(shown, person, values, []),
        Presentation.resolve(visiting, { ...person, chain: [] }, [], []),
    ];

    expect(presented.map(({ style }) => style[background])).toEqual(
        [destackTheme, new Theme(chosen), own].map(
            (each) => each.variables(DEFAULT_DISPLAY)[background],
        ),
    );
});

test("remember a person's last presentation per installation, and miss an unknown or older one", () => {
    // keep a presentation for the person, then read it back, for another person and from an older shape
    const storage = new SimulatedStorage();
    const presented = Presentation.resolve(page, { locale: "de-AT", chain: [] }, [], []);
    const missed = Presentation.cached(page, storage);
    Presentation.cache(page, presented, storage);
    const other = {
        ...page,
        context: {
            ...page.context,
            user: principal.user.reference(
                "universe",
                schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000e9"),
            ),
        },
    };
    storage.setItem(
        `destack:presentation:1:${page.context.installation}:${other.context.user.id}`,
        '{"locale":"de"}',
    );

    // expect the person's own back, nothing before it, and a miss for the other's older shape
    expect({
        missed,
        remembered: Presentation.cached(page, storage),
        other: Presentation.cached(other, storage),
    }).toEqual({ missed: undefined, remembered: presented, other: undefined });
});

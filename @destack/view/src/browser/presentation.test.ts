import { testSimulation } from "@destack/test/world";
import { TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { type JsonValue, schema } from "@destack/schema";
import { type Setting } from "@destack/setting";
import { deviceSetting, setting } from "@destack/setting/object";
import { DEFAULT_DISPLAY } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { accessRelationship, principal } from "@destack/access";
import { expect, onTestFinished, test } from "@destack/test";
import { CommandReference } from "../declare/command.ts";
import { keybindings, sources } from "../palette/index.ts";
import { appearance } from "../setting/index.ts";
import { Presentation, type Page } from "./presentation.ts";

/** The person a page presents for. */
const PERSON = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000e2");

/** The notes package whose view the page shows. */
const notes = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000e1",
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes home view's page, whose package declares two commands. */
const page: Page = {
    context: {
        installation: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000e3"),
        space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e4"),
        account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-0000000000e5"),
        view: "home",
        user: principal.user.reference("universe", PERSON),
        home: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e6"),
    },
    packageId: notes.id,
    theme: destackTheme,
    commands: [
        { packageId: notes.id, name: "complete", keybinding: "mod+enter" },
        { packageId: notes.id, name: "archive" },
    ],
};

test("present a page in its person's language, their display settings' style and their keybindings, offering none once they turn the package's commands off, and its device's own values over theirs", async () => {
    // place the person's dark appearance and their keybinding of the archive command, each under its own id
    await using simulation = testSimulation();
    const environment = simulation.environment("page");
    const storage = await TestDatabase.create(
        [accessRelationship, setting.table, deviceSetting.table],
        { environment, isMigrated: true },
    );
    onTestFinished(() => storage.close());
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
                    createdAt: now,
                    updatedAt: now,
                }),
        );
    };
    const archive = CommandReference.format({ packageId: notes.id, name: "archive" });
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
                createdAt: now,
                updatedAt: now,
            }),
    );
    const kept = await storage.database.select().from(deviceSetting.table);
    const onDevice = Presentation.resolve(page, person, await values(), kept);

    expect({
        presented,
        turnedOff: turnedOff.keybindings,
        onDevice: onDevice.style,
    }).toEqual({
        presented: {
            locale: "de-AT",
            style: destackTheme.variables({ ...DEFAULT_DISPLAY, appearance: "dark" }),
            keybindings: [
                { command: `${notes.id}/complete`, keybinding: "mod+enter" },
                { command: archive, keybinding: "mod+a" },
            ],
        },
        turnedOff: [],
        onDevice: destackTheme.variables({ ...DEFAULT_DISPLAY, appearance: "light" }),
    });
});

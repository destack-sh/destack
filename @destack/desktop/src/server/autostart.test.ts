import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { testSimulation } from "@destack/object/test";
import { Identifier } from "@destack/schema";
import { deviceSetting } from "@destack/setting/object";
import { expect, test } from "@destack/test";
import { autostart } from "../setting/index.ts";
import { localObjects, recordShell } from "../test/index.ts";
import { Autostart } from "./autostart.ts";
import { Menu } from "./menu.ts";

/** The tray's toggle, checked or not. */
function toggle(isChecked: boolean) {
    const item = { kind: "toggle", id: "autostart.toggle", title: "Open at Login", isChecked };

    return { action: "menu.set", menu: "tray", items: [item] };
}

test("open the desktop at login by default, follow this computer's value and toggle it from the tray", async () => {
    // run in a separate world
    const world = testSimulation().environment("test");

    // store this computer's settings in memory, recording the values the tray sets
    const device = Identifier.create("device", world);
    const settings = await localObjects({ deviceSetting }, world, device);
    const set: unknown[] = [];
    const { shell, sent } = recordShell();
    const menu = new Menu(shell, Localization.of(SOURCE_LOCALE, []));
    const atLogin = new Autostart(
        {
            settings,
            id: device,
            set: async (declared, value) => void set.push([declared.name, value]),
        },
        menu,
        shell,
        Localization.of(SOURCE_LOCALE, []),
    );

    // apply the default, then this computer's value once it is stored
    const show = async () =>
        atLogin.show(await settings.query.deviceSetting.findMany({ where: autostart.reference }));
    await show();
    const release = autostart.package.version;
    await settings.mutate(deviceSetting).create({ ...autostart.reference, value: false, release })
        .confirmed;
    await show();

    // toggle the applied value from the tray
    await menu.choose("autostart.toggle");

    // expect the registration and the checked item at each value, and the toggle's opposite value
    expect({ sent, set }).toEqual({
        sent: [
            { action: "autostart.set", isEnabled: true },
            toggle(true),
            { action: "autostart.set", isEnabled: false },
            toggle(false),
        ],
        set: [["autostart", true]],
    });
});

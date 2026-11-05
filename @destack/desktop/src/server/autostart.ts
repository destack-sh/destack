import { type Localization, t } from "@destack/locale";
import type { Select } from "@destack/db";
import { deviceSetting } from "@destack/setting/object";
import type { Device } from "@destack/machine/device";
import { autostart } from "../setting/index.ts";
import type { Shell } from "../shell/index.ts";
import type { Menu } from "./menu.ts";

/** The device autostart reads and sets: its settings and its identity. */
export type AutostartDevice = Pick<Device, "settings" | "set" | "id">;

/** Open the desktop at login as this computer's `autostart` setting says, offering its toggle. */
export class Autostart {
    /** The native device storing the setting. */
    readonly #native: AutostartDevice;
    /** The menus showing the toggle. */
    readonly #menu: Menu;
    /** The native shell registering the desktop at login. */
    readonly #shell: Shell;
    /** The language the toggle shows in. */
    readonly #localization: Localization;

    /** Apply the setting through the shell and show it in the menus, in a language. */
    constructor(native: AutostartDevice, menu: Menu, shell: Shell, localization: Localization) {
        // set the device, the menus, the shell and the language
        this.#native = native;
        this.#menu = menu;
        this.#shell = shell;
        this.#localization = localization;
    }

    /** Show the toggle as this computer's setting values say, at every change, until aborted. */
    async follow(signal: AbortSignal): Promise<void> {
        // watch this computer's values of the setting
        const query = this.#native.settings.query.deviceSetting.findMany({
            where: autostart.reference,
        });
        for await (const values of query.watch(signal)) {
            await this.show(values);
        }
    }

    /** Register the desktop at login as the values say, and show the toggle checked as it. */
    async show(values: readonly Select<typeof deviceSetting.table>[]): Promise<void> {
        // resolve this computer's value along a chain of the device alone
        const device = this.#native.id;
        const selection = { scope: device };
        const chain = [device];
        const isEnabled = autostart.resolve(selection, [], chain, values).value;

        // apply it, and offer its opposite
        await this.#shell.send({ action: "autostart.set", isEnabled });
        await this.#menu.set("autostart", [
            {
                kind: "toggle",
                id: "toggle",
                title: this.#localization.render(t`Open at Login`),
                isChecked: isEnabled,
                run: () => this.#native.set(autostart, !isEnabled),
            },
        ]);
    }
}

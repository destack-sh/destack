import { Snapshot } from "@destack/db";
import type { Identifier } from "@destack/schema";
import type { ObjectClient } from "@destack/object/client";
import { type setting, SettingValue } from "@destack/setting/object";
import { shortcut } from "@destack/view/declare";
import type { Shell } from "../shell/index.ts";

/** A global shortcut the shell reports by identity, its keys the person's `shortcut` setting. */
export class Shortcut {
    /** The identity a press reports. */
    readonly id: string;
    /** The native shell registering it. */
    readonly #shell: Shell;
    /** The keys registered, absent before the first registration. */
    #keys: string | undefined;

    /** Register a shortcut under an identity through the shell. */
    constructor(id: string, shell: Shell) {
        this.id = id;
        this.#shell = shell;
    }

    /** Register the keys the person's setting resolves to, at every change until aborted. */
    async follow(
        settings: ObjectClient<{ readonly setting: typeof setting }>,
        person: Identifier<"user">,
        signal: AbortSignal,
    ): Promise<void> {
        // watch the shortcut's values, which arrive once the replica has the person's access rows
        const selection = { scope: person };
        const where = shortcut.condition(selection);
        const watched = settings.query.setting.findMany({ where }).watch(signal);

        // register the keys the values along the person's chain resolve to, at each change
        for await (const values of watched) {
            const chain = await SettingValue.chain(Snapshot.live(settings.database), person);
            await this.register(shortcut.resolve(selection, values, chain).value);
        }
    }

    /** Register keys, leaving keys registered already, and remember them once sent. */
    async register(keys: string): Promise<void> {
        // send only keys that changed
        if (keys !== this.#keys) {
            await this.#shell.send({ action: "shortcut.register", id: this.id, shortcut: keys });
            this.#keys = keys;
        }
    }

    /** Release the shortcut, as nobody is signed in to press it. */
    async clear(): Promise<void> {
        // forget the keys, and release them
        this.#keys = undefined;
        await this.#shell.send({ action: "shortcut.unregister", id: this.id });
    }
}

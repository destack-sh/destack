import type { Localization } from "@destack/locale";
import type { Command } from "@destack/object";
import type { ObjectClient } from "@destack/object/client";
import { present } from "@destack/schema";
import type { MenuItem, MenuName, Shell } from "../shell/index.ts";
import { DesktopError } from "../error/index.ts";

/** The sections of the host's menus, each of which one render fills. */
const MENU_SECTIONS = ["serving", "settings", "autostart", "update", "account"] as const;

/** A section of the host's menus that one render fills. */
export type MenuSection = (typeof MENU_SECTIONS)[number];

/** The groups of sections each menu shows, in order, with a separator between groups. */
const MENUS: ReadonlyMap<MenuName, readonly (readonly MenuSection[])[]> = new Map([
    ["tray", [["serving"], ["settings", "autostart", "update"], ["account"]]],
    ["application", [["settings", "update"]]],
]);

/** An entry of a menu section: a line of text, an action, or a toggle with its check mark. */
export type MenuEntry =
    | {
          /** A line the person reads but cannot choose. */
          readonly kind: "label";
          /** The text. */
          readonly title: string;
      }
    | {
          /** An item the person chooses to run an action. */
          readonly kind: "action";
          /** The entry's identity within its section, the same across renders. */
          readonly id: string;
          /** The label. */
          readonly title: string;
          /** The key combination choosing it, such as `mod+,`. */
          readonly shortcut?: string;
          /** Run the action. */
          readonly run: () => Promise<void>;
      }
    | {
          /** An item with a check mark the person toggles. */
          readonly kind: "toggle";
          /** The entry's identity within its section, the same across renders. */
          readonly id: string;
          /** The label. */
          readonly title: string;
          /** Whether the item shows its check mark. */
          readonly isChecked: boolean;
          /** Toggle it. */
          readonly run: () => Promise<void>;
      };

/** A command a section offers on a followed row, and what runs once it is called. */
export interface MenuCommand {
    /** The command. */
    readonly command: Command;
    /** Run once the call confirms, on the row's identity. */
    readonly after?: (id: string) => Promise<void>;
}

/** The host's entries in the tray and the application menu, filled section by section. */
export class Menu {
    /** The native shell showing the menus. */
    readonly #shell: Shell;
    /** The entries of each section, absent for an empty one. */
    readonly #sections: Map<MenuSection, readonly MenuEntry[]>;
    /** The language commands' titles show in. */
    readonly #localization: Localization;
    /** The items each menu shows now, as last sent, by menu. */
    readonly #shown: Map<MenuName, string>;

    /** Fill the menus through the shell in a language. */
    constructor(shell: Shell, localization: Localization) {
        // start with every section and menu empty
        this.#shell = shell;
        this.#sections = new Map<MenuSection, readonly MenuEntry[]>();
        this.#localization = localization;
        this.#shown = new Map<MenuName, string>();
    }

    /** List a menu's items from its sections' entries, separating the non-empty groups. */
    static items(
        menu: MenuName,
        sections: ReadonlyMap<MenuSection, readonly MenuEntry[]>,
    ): MenuItem[] {
        // list each group's items, leaving out empty groups
        const groups = present(MENUS.get(menu), `the ${menu} menu`)
            .map((group) =>
                group.flatMap((section) =>
                    (sections.get(section) ?? []).map((entry, position) =>
                        Menu.#item(section, entry, position),
                    ),
                ),
            )
            .filter((items) => items.length > 0);

        return groups.flatMap((items, position) =>
            position === 0 ? items : [{ kind: "separator" as const }, ...items],
        );
    }

    /** List a command's action on a target, none where the target misses its condition. */
    async offer(
        offered: MenuCommand,
        client: Pick<ObjectClient, "queryType" | "call">,
        id: string,
    ): Promise<MenuEntry[]> {
        // leave out a command whose condition the target misses, or which the person may not call
        const { command, after } = offered;
        const target = await client.queryType(command.object).findFirst(command.query([id]));
        if (target === undefined) {
            return [];
        }

        // call it on the target, then run what follows it
        const run = async () => {
            await client.call(command.object, command.method, command.input({ id })).confirmed;
            await after?.(id);
        };
        const title = this.#localization.render(command.title);

        return [{ kind: "action", id: command.method, title, run }];
    }

    /** Offer a watched row's commands in a section at each change, none without a row. */
    async follow(
        section: MenuSection,
        client: Pick<ObjectClient, "queryType" | "call">,
        rows: AsyncIterable<{ readonly id: string } | undefined>,
        commands: readonly MenuCommand[],
    ): Promise<void> {
        for await (const found of rows) {
            // offer each command applying to the row, nothing without one
            const entries =
                found === undefined
                    ? []
                    : await Promise.all(
                          commands.map((offered) => this.offer(offered, client, found.id)),
                      );
            await this.set(section, entries.flat());
        }
    }

    /** Replace a section's entries and send each menu showing it. */
    async set(section: MenuSection, entries: readonly MenuEntry[]): Promise<void> {
        // set the section's entries
        this.#sections.set(section, entries);

        // send each menu showing the section once its items changed
        for (const [menu, groups] of MENUS) {
            const items = Menu.items(menu, this.#sections);
            const shown = JSON.stringify(items);
            if (
                groups.some((group) => group.includes(section)) &&
                this.#shown.get(menu) !== shown
            ) {
                this.#shown.set(menu, shown);
                await this.#shell.send({ action: "menu.set", menu, items });
            }
        }
    }

    /** Run the entry of an item the person chose, refusing one the menus no longer show. */
    async choose(id: string): Promise<void> {
        // find the entry its section shows under the identity
        const entry = MENU_SECTIONS.flatMap((section) =>
            (this.#sections.get(section) ?? []).filter(
                (shown) => shown.kind !== "label" && `${section}.${shown.id}` === id,
            ),
        ).at(0);

        // refuse an item no section shows, then run it
        if (entry === undefined || entry.kind === "label") {
            throw DesktopError.of("MENU_ITEM_NOT_FOUND", `no menu item ${id} runs`);
        }
        await entry.run();
    }

    /** Describe an entry as the shell shows it, under its section's identity. */
    static #item(section: MenuSection, entry: MenuEntry, position: number): MenuItem {
        // show a label under its position, as nobody chooses it
        if (entry.kind === "label") {
            return { kind: "label", id: `${section}.label.${position}`, title: entry.title };
        }
        // show a toggle with its check mark
        else if (entry.kind === "toggle") {
            const { title, isChecked } = entry;

            return { kind: "toggle", id: `${section}.${entry.id}`, title, isChecked };
        }
        // show an action with its key combination
        else {
            const { title, shortcut } = entry;

            return {
                kind: "action",
                id: `${section}.${entry.id}`,
                title,
                ...(shortcut === undefined ? {} : { shortcut }),
            };
        }
    }
}

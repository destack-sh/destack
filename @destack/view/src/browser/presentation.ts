import type { LocaleTag } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import type { DeviceSettingValue, SettingValue } from "@destack/setting/object";
import { type Theme } from "@destack/theme";
import { CommandReference } from "../declare/command.ts";
import type { ViewContext } from "../declare/context.ts";
import type { CommandDeclaration } from "../declare/command.ts";
import type { Keybinding } from "../palette/accelerator.ts";
import { keybindings, Palette, sources } from "../palette/index.ts";
import { DISPLAY_SETTINGS, resolveDisplay } from "../setting/display.ts";

/** What a page presents for its person: their language, the theme's style and their keybindings. */
export interface Presentation {
    /** The language and region the person reads. */
    readonly locale: LocaleTag;
    /** The theme's custom properties and color scheme for the person's display settings. */
    readonly style: Readonly<Record<string, string>>;
    /** The key combinations running the installation's commands, for the person. */
    readonly keybindings: readonly Keybinding[];
}

/** The settings a presentation reads in the person's own scope. */
export const PRESENTED_SETTINGS = [...DISPLAY_SETTINGS, keybindings.reference, sources.reference];

/** A page as its person reads it: its context, package, theme and declared commands. */
export interface Page {
    /** Where and for whom the view runs. */
    readonly context: ViewContext;
    /** The package serving the view. */
    readonly packageId: PackageId;
    /** The theme the page shows in. */
    readonly theme: Theme;
    /** The commands the package declares, with their key combinations. */
    readonly commands: readonly CommandDeclaration[];
}

/** The resolution of a page's presentation from its person's settings and language. */
export const Presentation = {
    /** Resolve a page's presentation from its person's settings and language. */
    resolve(
        page: Page,
        person: { readonly locale: LocaleTag; readonly chain: readonly string[] },
        values: readonly SettingValue[],
        device: readonly DeviceSettingValue[],
    ): Presentation {
        // select the person in the page's space and its package's installation
        const { context, packageId, theme } = page;
        const selection = {
            scope: context.user.id,
            package: packageId,
            space: schema.identifier("space").parse(context.space),
            installation: schema.identifier("installation").parse(context.installation),
        };

        // resolve the display, and the keybindings of the commands unless the person turned the package off
        const { chain } = person;
        const display = resolveDisplay(selection, values, chain, device);
        const overrides = keybindings.resolve(selection, values, chain, device).value;
        const isOffered =
            sources.resolve(selection, values, chain, device).value[packageId] !== false;
        const bound = page.commands.flatMap((command) => {
            const keybinding = Palette.keybinding(command, command.keybinding, overrides);

            return keybinding === null
                ? []
                : [{ command: CommandReference.format(command), keybinding }];
        });

        return {
            locale: person.locale,
            style: theme.variables(display),
            keybindings: isOffered ? bound : [],
        };
    },
};

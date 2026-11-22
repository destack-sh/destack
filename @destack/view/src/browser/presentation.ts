import type { LocaleTag, Shortcut } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import type { DeviceSettingValue, SettingValue } from "@destack/setting/object";
import { type Theme } from "@destack/theme";
import type { ViewContext } from "../declare/context.ts";
import { keybindings, sources } from "../palette/index.ts";
import { DISPLAY_SETTINGS, resolveDisplay } from "../setting/display.ts";

/** What a page presents for its person: their language, the theme's style and their keybindings. */
export interface Presentation {
    /** The language and region the person reads. */
    readonly locale: LocaleTag;
    /** The theme's custom properties and color scheme for the person's display settings. */
    readonly style: Readonly<Record<string, string>>;
    /** The person's key combinations by action key, null for one turned off. */
    readonly keybindings: Readonly<Record<string, Shortcut | null>>;
    /** Whether the person offers the package's actions. */
    readonly isOffered: boolean;
}

/** The settings a presentation reads in the person's own scope. */
export const PRESENTED_SETTINGS = [...DISPLAY_SETTINGS, keybindings.reference, sources.reference];

/** A page as its person reads it: its context, package and theme. */
export interface Page {
    /** Where and for whom the view runs. */
    readonly context: ViewContext;
    /** The package serving the view. */
    readonly packageId: PackageId;
    /** The theme the page shows in. */
    readonly theme: Theme;
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
        // select the person, or an anonymous visitor's defaults, in the page's space and its package's installation
        const { context, packageId, theme } = page;
        const selection = {
            scope: principal.user.is(context.user) ? context.user.id : null,
            package: packageId,
            space: schema.identifier("space").parse(context.space),
            installation: schema.identifier("installation").parse(context.installation),
        };

        // resolve the display, the person's key combinations, and whether they offer the package's actions
        const { chain } = person;
        const display = resolveDisplay(selection, values, chain, device);
        const offered = sources.resolve(selection, values, chain, device).value;

        return {
            locale: person.locale,
            style: theme.variables(display),
            keybindings: keybindings.resolve(selection, values, chain, device).value,
            isOffered: offered[packageId] !== false,
        };
    },
};

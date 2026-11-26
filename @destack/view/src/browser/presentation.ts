import type { Storage } from "@destack/environment/browser";
import { LocaleTag, Shortcut } from "@destack/locale";
import type { PackageId } from "@destack/package";
import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import type { DeviceSettingValue, SettingValue } from "@destack/setting/object";
import { Theme } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import type { ViewContext } from "../declare/context.ts";
import { keybindings, sources } from "../palette/index.ts";
import { DISPLAY_SETTINGS, resolveDisplay, theme } from "../setting/display.ts";

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

/** The key prefix of a page's last presentation in browser storage, versioned by shape. */
const CACHE_KEY = "destack:presentation:1";

/** A presentation as the browser's storage keeps it. */
const CachedPresentation = schema.object({
    /** The language and region the person reads. */
    locale: LocaleTag,
    /** The theme's custom properties and color scheme. */
    style: schema.record(schema.string(), schema.string()),
    /** The person's key combinations by action key, null for one turned off. */
    keybindings: schema.record(schema.string(), Shortcut.nullable()),
    /** Whether the person offers the package's actions. */
    isOffered: schema.boolean(),
});

/** The settings a presentation reads in the person's own scope. */
export const PRESENTED_SETTINGS = [...DISPLAY_SETTINGS, keybindings.reference, sources.reference];

/** A page as its person reads it: its context, package and theme. */
export interface Page {
    /** Where and for whom the view runs. */
    readonly context: ViewContext;
    /** The package serving the view. */
    readonly packageId: PackageId;
    /** The theme the page shows visitors in. */
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
        const { context, packageId } = page;
        const isPerson = principal.user.is(context.user);
        const selection = {
            scope: isPerson ? context.user.id : null,
            package: packageId,
            space: schema.identifier("space").parse(context.space),
            installation: schema.identifier("installation").parse(context.installation),
        };

        // resolve the display, the person's key combinations, and whether they offer the package's actions
        const { chain } = person;
        const display = resolveDisplay(selection, values, chain, device);
        const offered = sources.resolve(selection, values, chain, device).value;

        // show a person their own theme, else Destack's, and a visitor the page's
        const chosen = theme.resolve(selection, values, chain, device).value;
        const own = chosen === null ? destackTheme : new Theme(chosen);
        const shown = isPerson ? own : page.theme;

        return {
            locale: person.locale,
            style: shown.variables(display),
            keybindings: keybindings.resolve(selection, values, chain, device).value,
            isOffered: offered[packageId] !== false,
        };
    },

    /** Read the page's last presentation for its person in the browser, absent before the first. */
    cached(page: Page, storage: Storage): Presentation | undefined {
        // read the stored presentation, a missing or older one a miss the device answers
        const stored = storage.getItem(cacheKey(page));
        const parsed =
            stored === null ? undefined : CachedPresentation.safeParse(JSON.parse(stored));

        return parsed?.success === true ? parsed.data : undefined;
    },

    /** Keep the page's presentation for its person in the browser, for its next load. */
    cache(page: Page, presentation: Presentation, storage: Storage): void {
        storage.setItem(cacheKey(page), JSON.stringify(presentation));
    },
};

/** Name a page's presentation in the browser's storage, by its installation and person. */
function cacheKey(page: Page): string {
    return `${CACHE_KEY}:${page.context.installation}:${page.context.user.id}`;
}

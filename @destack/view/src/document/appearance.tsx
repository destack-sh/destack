import type { Appearance } from "@destack/theme";
import type { JSX } from "@solidjs/web";
import { createPrefersDark } from "../primitives/media.ts";
import { createMutationObserver } from "../primitives/mutation-observer.ts";
import { onMount } from "../primitives/utils.ts";
import { type Accessor, createMemo, createSignal } from "../solid/reactive.ts";

/** The browser storage key a page keeps a visitor's appearance under. */
const STORAGE_KEY = "destack-appearance";

/** A color scheme a page renders in. */
export type ColorScheme = "light" | "dark";

/** Where a visitor's appearance is stored, and the appearance before they choose one. */
export interface AppearanceOptions {
    /** The browser storage key the appearance sits under, `destack-appearance` by default. */
    readonly storageKey?: string;
    /** The appearance before the visitor chooses one, the system's by default. */
    readonly defaultAppearance?: Appearance;
}

/** Restore a visitor's stored appearance before the page paints, for a page without an account's settings. */
export function AppearanceScript(properties: AppearanceOptions): JSX.Element {
    // set the stored or default scheme on the root, keeping the system's when storage holds none or refuses
    const key = JSON.stringify(properties.storageKey ?? STORAGE_KEY);
    const fallback = JSON.stringify(properties.defaultAppearance ?? "system");
    const code = `try{const a=localStorage.getItem(${key})??${fallback};if(a==="light"||a==="dark")document.documentElement.style.colorScheme=a}catch{}`;

    return <script innerHTML={code} />;
}

/** A visitor's appearance, kept in browser storage, with the scheme it resolves to and the system's. */
export interface LocalAppearance {
    /** The visitor's appearance: the system's, light or dark. */
    readonly appearance: Accessor<Appearance>;
    /** Set and store the visitor's appearance, and apply it to the page. */
    readonly setAppearance: (appearance: Appearance) => void;
    /** The scheme the page renders in. */
    readonly resolvedAppearance: Accessor<ColorScheme>;
    /** The scheme the system prefers. */
    readonly systemAppearance: Accessor<ColorScheme>;
}

/** Follow and set a visitor's appearance in browser storage, for a page without an account's settings. */
export function createAppearance(options: AppearanceOptions = {}): LocalAppearance {
    // hold the stored appearance and follow the device's scheme
    const { storageKey = STORAGE_KEY, defaultAppearance = "system" } = options;
    const [appearance, setStored] = createSignal<Appearance>(defaultAppearance, {
        ownedWrite: true,
    });
    const isDeviceDark = createPrefersDark();
    const systemAppearance = (): ColorScheme => (isDeviceDark() ? "dark" : "light");

    // restore the stored choice once the page runs
    onMount(() => {
        const stored = localStorage.getItem(storageKey);
        setStored(
            stored === "light" || stored === "dark" || stored === "system"
                ? stored
                : defaultAppearance,
        );
    });

    return {
        appearance,
        setAppearance(next) {
            // store the choice, then set the root's scheme, the system's leaving it to the device
            localStorage.setItem(storageKey, next);
            document.documentElement.style.colorScheme = next === "system" ? "light dark" : next;
            setStored(next);
        },
        resolvedAppearance: () => {
            const chosen = appearance();

            return chosen === "system" ? systemAppearance() : chosen;
        },
        systemAppearance,
    };
}

/** Follow the scheme an element renders in: its own or an enclosing theme root's, else the device's. */
export function createColorScheme(element: Accessor<Element | undefined>): Accessor<ColorScheme> {
    // follow the device's scheme and every theme root's style above and at the element
    const isDeviceDark = createPrefersDark();
    const [mutations, setMutations] = createSignal(0, { ownedWrite: true });
    createMutationObserver(
        () => rootsOf(element()),
        { attributes: true, attributeFilter: ["style", "class"] },
        () => setMutations((count) => count + 1),
    );

    // read the computed scheme, deferring to the device where the element allows both
    return createMemo(() => {
        // read the element once a theme root or the device changed
        mutations();
        const target = element();
        if (target === undefined) {
            return isDeviceDark() ? "dark" : "light";
        }
        const allowed = getComputedStyle(target).colorScheme;
        const isDark = allowed.includes("dark") && (!allowed.includes("light") || isDeviceDark());

        return isDark ? "dark" : "light";
    });
}

/** List an element and the elements enclosing it, the theme roots that may set its scheme. */
function rootsOf(element: Element | undefined): Element[] {
    const roots: Element[] = [];
    for (let root = element ?? null; root !== null; root = root.parentElement) {
        roots.push(root);
    }

    return roots;
}

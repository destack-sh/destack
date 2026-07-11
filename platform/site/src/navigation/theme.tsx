import { createSignal, onCleanup, onMount } from "solid-js";

import { commandEvents } from "../command/command";
import { ShortcutLabel } from "./shortcut";

/// The browser storage key for the selected theme.
const themeStorageKey = "destack-theme";

/// One supported site theme.
type Theme = "dark" | "light";

/// Set the theme before page assets paint.
export const themeInitializer = `
    (() => {
        const stored = localStorage.getItem("${themeStorageKey}");
        const system = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
        document.documentElement.dataset.theme =
            stored === "dark" || stored === "light" ? stored : system;
    })();
`;

/// Render the persistent site theme toggle.
export function ThemeToggle() {
    const [theme, setTheme] = createSignal<Theme>("light");
    const nextTheme = () => (theme() === "light" ? "dark" : "light");

    const selectNextTheme = () => {
        const selected = nextTheme();

        document.documentElement.dataset.theme = selected;
        localStorage.setItem(themeStorageKey, selected);
        setTheme(selected);
    };

    // synchronize with the pre-paint theme after hydration
    onMount(() => {
        setTheme(document.documentElement.dataset.theme === "dark" ? "dark" : "light");
        document.addEventListener(commandEvents.toggleTheme, selectNextTheme);
        onCleanup(() => document.removeEventListener(commandEvents.toggleTheme, selectNextTheme));
    });

    return (
        <button
            aria-label={`Use ${nextTheme()} theme`}
            class="site-theme-toggle"
            data-shortcut="t"
            onClick={selectNextTheme}
            title={`Alt+T: use ${nextTheme()} theme`}
            type="button"
        >
            [<ShortcutLabel brackets={false} label={`theme:${nextTheme()}`} shortcut="t" />]
        </button>
    );
}

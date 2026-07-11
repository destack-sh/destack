import { createSignal, onCleanup, onMount } from "solid-js";

import { commandEvents } from "../command/command";
import { ShortcutLabel } from "./shortcut";

/// The browser storage key for the selected theme.
const themeStorageKey = "destack-theme";

/// The ordered theme preferences cycled by the toggle.
const themePreferences = ["system", "light", "dark"] as const;

/// One supported site theme.
type Theme = "dark" | "light";

/// One persistent theme preference.
type ThemePreference = (typeof themePreferences)[number];

/// Set the theme before page assets paint.
export const themeInitializer = `
    (() => {
        const stored = localStorage.getItem("${themeStorageKey}");
        const preference = stored === "dark" || stored === "light" ? stored : "system";
        const systemTheme = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";

        document.documentElement.dataset.theme = preference === "system" ? systemTheme : preference;
        document.documentElement.dataset.themePreference = preference;
    })();
`;

/// Render the persistent site theme toggle.
export function ThemeToggle() {
    const [preference, setPreference] = createSignal<ThemePreference>("system");
    const nextPreference = () => {
        const index = themePreferences.indexOf(preference());

        return themePreferences[(index + 1) % themePreferences.length];
    };

    const selectPreference = (selected: ThemePreference) => {
        document.documentElement.dataset.theme = resolveTheme(selected);
        document.documentElement.dataset.themePreference = selected;

        if (selected === "system") {
            localStorage.removeItem(themeStorageKey);
        } else {
            localStorage.setItem(themeStorageKey, selected);
        }

        setPreference(selected);
    };
    const selectNextPreference = () => selectPreference(nextPreference());

    // synchronize with the pre-paint theme after hydration
    onMount(() => {
        const selected = document.documentElement.dataset.themePreference;
        if (!isThemePreference(selected)) {
            throw new Error(`invalid theme preference: ${selected}`);
        }
        setPreference(selected);

        // follow operating-system changes while the system preference is active
        const systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
        const synchronizeSystemTheme = () => {
            if (preference() === "system") {
                document.documentElement.dataset.theme = systemTheme.matches ? "dark" : "light";
            }
        };

        systemTheme.addEventListener("change", synchronizeSystemTheme);
        document.addEventListener(commandEvents.toggleTheme, selectNextPreference);
        onCleanup(() => {
            systemTheme.removeEventListener("change", synchronizeSystemTheme);
            document.removeEventListener(commandEvents.toggleTheme, selectNextPreference);
        });
    });

    return (
        <button
            aria-label={`Theme: ${preference()}; select ${nextPreference()}`}
            class="site-theme-toggle"
            data-shortcut="t"
            onClick={selectNextPreference}
            title={`Alt+T: select ${nextPreference()} theme`}
            type="button"
        >
            [<ShortcutLabel brackets={false} label={`theme:${preference()}`} shortcut="t" />]
        </button>
    );
}

/// Resolve one preference to its active light or dark theme.
function resolveTheme(preference: ThemePreference): Theme {
    if (preference === "system") {
        return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }

    return preference;
}

/// Return whether one document value is a supported theme preference.
function isThemePreference(value?: string): value is ThemePreference {
    return themePreferences.some((preference) => preference === value);
}

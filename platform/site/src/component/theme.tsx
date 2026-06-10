import { createSignal, onCleanup, onMount } from "solid-js";

type Theme = "light" | "dark";

const themeEvent = "destack-theme";

/// Flip the site theme and persist the explicit choice.
export function toggleTheme() {
    const next = currentTheme() === "dark" ? "light" : "dark";
    localStorage.setItem("destack-theme", next);
    applyTheme(next);
}

export function ThemeToggle() {
    const [theme, setTheme] = createSignal<Theme>("light");

    onMount(() => {
        setTheme(currentTheme());

        // stay in sync when the theme changes elsewhere (command bar, other toggles)
        const syncTheme = () => setTheme(currentTheme());
        window.addEventListener(themeEvent, syncTheme);
        onCleanup(() => window.removeEventListener(themeEvent, syncTheme));

        // follow system changes while the user has not chosen explicitly
        const media = window.matchMedia("(prefers-color-scheme: dark)");
        const followSystem = () => {
            if (localStorage.getItem("destack-theme") == undefined) {
                applyTheme(media.matches ? "dark" : "light");
            }
        };
        media.addEventListener("change", followSystem);
        onCleanup(() => media.removeEventListener("change", followSystem));
    });

    return (
        <button
            aria-label={theme() === "dark" ? "switch to light mode" : "switch to dark mode"}
            class="hover:text-destack-accent"
            onClick={toggleTheme}
            type="button"
        >
            {theme() === "dark" ? "☾" : "☀"}
        </button>
    );
}

function currentTheme(): Theme {
    return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

function applyTheme(theme: Theme) {
    document.documentElement.dataset.theme = theme;
    window.dispatchEvent(new Event(themeEvent));
}

import { createSignal, onMount } from "solid-js";

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

    // synchronize with the pre-paint theme after hydration
    onMount(() => {
        setTheme(document.documentElement.dataset.theme === "dark" ? "dark" : "light");
    });

    return (
        <button
            aria-label={`Use ${nextTheme()} theme`}
            class="site-theme-toggle"
            onClick={() => {
                const selected = nextTheme();

                document.documentElement.dataset.theme = selected;
                localStorage.setItem(themeStorageKey, selected);
                setTheme(selected);
            }}
            title={`Use ${nextTheme()} theme`}
            type="button"
        >
            [{nextTheme()}]
        </button>
    );
}

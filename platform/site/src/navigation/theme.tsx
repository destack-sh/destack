import { color } from "@destack/theme/tokens.stylex";
import { createSignal, onSettled, Show } from "@destack/view";
import * as stylex from "@destack/style";

/// Switch between light and dark themes, starting with the system preference.
export function ThemeToggle() {
    const [isDark, setIsDark] = createSignal(false);

    // follow system changes until the reader selects a theme
    onSettled(() => {
        const preference = window.matchMedia("(prefers-color-scheme: dark)");
        const update = () => {
            const theme = document.documentElement.dataset.theme;
            setIsDark(theme === "dark" || (theme === undefined && preference.matches));
        };
        update();
        preference.addEventListener("change", update);
        return () => preference.removeEventListener("change", update);
    });

    // apply the choice immediately and retain it for the next visit
    const toggle = () => {
        const theme = isDark() ? "light" : "dark";
        document.documentElement.dataset.theme = theme;
        document.documentElement.dataset.destackTheme = theme;
        document.documentElement.style.colorScheme = theme;
        setIsDark(theme === "dark");
        try {
            localStorage.setItem("destack-theme", theme);
        } catch (error) {
            console.warn("Could not save the site theme", error);
        }
    };

    return (
        <button
            aria-label={isDark() ? "Switch to light theme" : "Switch to dark theme"}
            title={isDark() ? "Switch to light theme" : "Switch to dark theme"}
            onClick={toggle}
            type="button"
            {...stylex.attrs(styles.toggle)}
        >
            <svg
                aria-hidden="true"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <Show
                    when={isDark()}
                    fallback={<path d="M20.5 13A8.5 8.5 0 0 1 11 3.5 8.5 8.5 0 1 0 20.5 13Z" />}
                >
                    <circle cx="12" cy="12" r="4" />
                    <path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5" />
                </Show>
            </svg>
        </button>
    );
}

const styles = stylex.create({
    toggle: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        display: "inline-flex",
        height: "2.75rem",
        justifyContent: "center",
        padding: 0,
        width: "2.75rem",
        ":hover": { color: color.primary },
    },
});

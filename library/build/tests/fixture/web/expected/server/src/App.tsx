import { createSignal, onSettled } from "@destack/view";
import { createTheme } from "@destack/theme";
import "@destack/theme/theme.css";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import "./style.css";

/** Shared styles compiled into the browser stylesheet. */
const styles = style.create({ title: { color: color.primary } });

/** Theme shared by the server and browser renders. */
const theme = createTheme({ appearance: "light", accent: "indigo" });

/** Render the build fixture. */
export default function App() {
    const [count, setCount] = createSignal(0);

    // install browser interactions after rendering
    onSettled(() => {
        document.addEventListener("click", focusHeading);
        return () => document.removeEventListener("click", focusHeading);
    });

    return (
        <main {...theme}>
            <h1 {...style.attrs(styles.title)}>Hello Destack</h1>
            <button onClick={() => setCount(count() + 1)}>Count: {count()}</button>
        </main>
    );
}

/** Focus the heading from a browser lifecycle callback. */
function focusHeading() {
    document.querySelector("h1")?.focus();
}

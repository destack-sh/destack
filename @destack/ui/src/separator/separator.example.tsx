import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { Separator } from "./separator.tsx";

/** The row of footer links. */
const styles = style.create({
    footer: { display: "flex", gap: space[3] },
});

/** Show a footer whose links a vertical line divides. */
export function SeparatorExample(): JSX.Element {
    return (
        <nav aria-label="Footer" {...style.attrs(styles.footer)}>
            <a href="/privacy">Privacy</a>
            <Separator orientation="vertical" />
            <a href="/terms">Terms</a>
        </nav>
    );
}

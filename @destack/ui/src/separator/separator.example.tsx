import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { Separator } from "./separator.tsx";

/** The row of footer links. */
const styles = style.create({
    footer: { display: "flex", gap: space[3] },
});

/** A footer whose links a vertical line divides. */
export const separatorFooterLinks = defineExample({
    of: Separator,
    name: "footer-links",
    description: "a footer whose links a vertical line divides",
    render: () => (
        <nav aria-label="Footer" {...style.attrs(styles.footer)}>
            <a href="/privacy">Privacy</a>
            <Separator orientation="vertical" />
            <a href="/terms">Terms</a>
        </nav>
    ),
});

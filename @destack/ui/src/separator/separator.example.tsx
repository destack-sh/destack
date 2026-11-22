import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { Stack } from "../stack/index.ts";
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
    properties: {
        orientation: "vertical",
    },
    render: (properties) => (
        <nav aria-label="Footer" {...style.attrs(styles.footer)}>
            <a href="/privacy">Privacy</a>
            <Separator {...properties} />
            <a href="/terms">Terms</a>
        </nav>
    ),
});

/** Two sections of a settings page that a horizontal line divides. */
export const separatorSections = defineExample({
    of: Separator,
    name: "sections",
    description: "two sections of a settings page that a horizontal line divides",
    properties: {
        orientation: "horizontal",
    },
    render: (properties) => (
        <Stack>
            <p>Account</p>
            <Separator {...properties} />
            <p>Notifications</p>
        </Stack>
    ),
});

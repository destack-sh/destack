import * as stylex from "@stylexjs/stylex";

import { CommandPalette } from "../command/palette";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";
import { styles } from "./topbar.stylex";

/// Render the global site navigation.
export function TopBar() {
    return (
        <header {...stylex.attrs(styles.root)}>
            <div {...stylex.attrs(styles.body)}>
                <SiteLink href="/" shortcut="h" style={styles.brand} title="Alt+H: home">
                    destack.sh
                </SiteLink>

                <nav aria-label="Primary navigation" {...stylex.attrs(styles.primary)}>
                    {primaryLinks.map(({ label, href, shortcut }) => (
                        <SiteLink
                            href={href}
                            shortcut={shortcut}
                            style={styles.link}
                            title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                        >
                            {label}
                        </SiteLink>
                    ))}
                </nav>

                <div {...stylex.attrs(styles.actions)}>
                    <CommandPalette />
                </div>
            </div>
        </header>
    );
}

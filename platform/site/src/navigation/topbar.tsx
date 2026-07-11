import { CommandPalette } from "../command/palette";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";
import { ShortcutLabel } from "./shortcut";
import { ThemeToggle } from "./theme";

/// Render the global site navigation.
export function TopBar() {
    return (
        <header class="site-topbar">
            <div class="site-topbar__body">
                <SiteLink class="site-topbar__brand" href="/" shortcut="h" title="Alt+H: home">
                    <ShortcutLabel brackets={false} label="destack.sh" shortcut="h" />
                </SiteLink>

                <nav aria-label="Primary navigation">
                    <CommandPalette />
                    {primaryLinks.map(({ label, href, shortcut }) => (
                        <SiteLink
                            href={href}
                            shortcut={shortcut}
                            title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                        >
                            <ShortcutLabel label={label} shortcut={shortcut} />
                        </SiteLink>
                    ))}
                    <ThemeToggle />
                </nav>
            </div>
        </header>
    );
}

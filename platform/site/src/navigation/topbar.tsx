import { CommandPalette } from "../command/palette";
import { SiteLink } from "./link";
import { primaryLinks } from "./navigation";

/// Render the global site navigation.
export function TopBar() {
    return (
        <header class="site-topbar">
            <div class="site-topbar__body">
                <SiteLink class="site-topbar__brand" href="/" shortcut="h" title="Alt+H: home">
                    destack.sh
                </SiteLink>

                <nav aria-label="Primary navigation" class="site-topbar__primary">
                    {primaryLinks.map(({ label, href, shortcut }) => (
                        <SiteLink
                            href={href}
                            shortcut={shortcut}
                            title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                        >
                            {label}
                        </SiteLink>
                    ))}
                </nav>

                <div class="site-topbar__actions">
                    <CommandPalette />
                </div>
            </div>
        </header>
    );
}

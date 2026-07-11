import { SiteLink } from "./link";
import { socialLinks } from "./navigation";
import { ShortcutLabel } from "./shortcut";

/// Render the global site footer.
export function Footer() {
    return (
        <footer class="site-footer">
            <div class="site-footer__body">
                <span class="site-footer__copyright">(c) symbol industries</span>

                <nav aria-label="Community links">
                    {socialLinks.map(({ label, href, shortcut }) => (
                        <SiteLink
                            href={href}
                            shortcut={shortcut}
                            title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                        >
                            <ShortcutLabel label={label} shortcut={shortcut} />
                        </SiteLink>
                    ))}
                </nav>
            </div>
        </footer>
    );
}

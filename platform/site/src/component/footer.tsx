import { A } from "@solidjs/router";

/// The public footer destinations.
const links = [
    ["blog", "/blog/"],
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["x", "https://x.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

/// Render the global site footer.
export function Footer() {
    return (
        <footer class="site-footer">
            <div class="site-footer__body">
                <span>destack.sh</span>
                <nav aria-label="Community links">
                    {links.map(([label, href]) =>
                        href.startsWith("/") ? (
                            <A href={href}>[{label}]</A>
                        ) : (
                            <a href={href}>[{label}]</a>
                        ),
                    )}
                </nav>
                <span>zurich, switzerland</span>
            </div>
        </footer>
    );
}

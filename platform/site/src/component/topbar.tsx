import { A } from "@solidjs/router";

/// The primary public navigation destinations.
const links = [
    ["design", "https://github.com/destack-sh/destack/blob/main/language/DESIGN.md"],
    ["blog", "/blog/"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

/// Render the global site navigation.
export function TopBar() {
    return (
        <header class="site-topbar">
            <div class="site-topbar__body">
                <A class="site-topbar__brand" href="/">
                    destack.sh
                </A>

                <nav aria-label="Primary navigation">
                    {links.map(([label, href]) =>
                        href.startsWith("/") ? (
                            <A href={href}>[{label}]</A>
                        ) : (
                            <a href={href}>[{label}]</a>
                        ),
                    )}
                </nav>
            </div>
        </header>
    );
}

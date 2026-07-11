import { A } from "@solidjs/router";

import { ThemeToggle } from "./theme-toggle";

/// The primary public navigation destinations.
const links = [
    ["blog", "/blog/"],
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["x", "https://x.com/destack"],
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
                    <ThemeToggle />
                </nav>
            </div>
        </header>
    );
}

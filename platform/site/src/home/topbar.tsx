import { A } from "@solidjs/router";

import { Icon } from "./icon";

const communityLinks = [
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["twitter", "https://twitter.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

export function TopBar() {
    return (
        <header class="border-b border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between px-4 md:px-10">
                {/* brand */}
                <A class="flex items-center gap-2 text-sm font-extrabold" href="/">
                    <Icon class="size-7" />
                    <span>destack</span>
                </A>

                {/* community links, hidden on phones, slash-separated otherwise */}
                <nav class="hidden items-center gap-2 text-sm font-extrabold lowercase sm:flex">
                    {communityLinks.map(([label, href], index) => (
                        <>
                            {index > 0 && <span class="text-destack-panel/40">/</span>}
                            <a class="hover:text-destack-accent" href={href}>
                                {label}
                            </a>
                        </>
                    ))}
                </nav>
            </div>
        </header>
    );
}

import { A } from "@solidjs/router";

import { Icon } from "./icon";
import { ThemeToggle } from "./theme";

const communityLinks = [
    ["blog", "/blog/"],
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["twitter", "https://twitter.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

export function TopBar() {
    return (
        <header class="border-b border-neutral-950 bg-destack-ink text-destack-cream">
            <div class="mx-auto grid h-12 max-w-328 grid-cols-[1fr_auto_1fr] items-center gap-3 px-4 md:px-10">
                <A class="flex min-w-0 items-center gap-2 text-sm font-extrabold" href="/">
                    <Icon class="size-7" />
                    <span>destack</span>
                </A>

                <p class="hidden min-w-0 truncate text-sm font-extrabold lowercase md:block">
                    program the universe
                </p>

                <nav class="flex shrink-0 items-center justify-end gap-1 text-xs font-extrabold lowercase sm:gap-2 sm:text-sm">
                    {communityLinks.map(([label, href], index) => (
                        <>
                            {index > 0 && <Separator />}
                            {href.startsWith("/") ? (
                                <A class="hover:text-destack-accent" href={href}>
                                    {label}
                                </A>
                            ) : (
                                <a class="hover:text-destack-accent" href={href}>
                                    {label}
                                </a>
                            )}
                        </>
                    ))}
                    <Separator />
                    <ThemeToggle />
                </nav>
            </div>
        </header>
    );
}

function Separator() {
    return <span class="text-destack-cream/35">·</span>;
}

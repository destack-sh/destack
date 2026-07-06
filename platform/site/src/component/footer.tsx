import { A } from "@solidjs/router";

const communityLinks = [
    ["blog", "/blog/"],
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["x", "https://x.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

export function Footer() {
    return (
        <footer class="border-t border-neutral-950 bg-destack-ink text-destack-cream">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between gap-4 px-4 text-sm font-extrabold lowercase md:px-10">
                <span class="truncate text-destack-cream">© Symbol Industries</span>

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
                </nav>
            </div>
        </footer>
    );
}

function Separator() {
    return <span class="text-destack-cream/35">·</span>;
}

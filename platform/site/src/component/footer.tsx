import { A } from "@solidjs/router";

const communityLinks = [
    ["blog", "/blog/"],
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["x", "https://x.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

export function Footer() {
    return (
        <footer class="border-t border-destack-cream/20 bg-destack-ink text-destack-cream">
            <div class="flex h-14 max-w-[22rem] items-center justify-between gap-4 px-4 text-xs font-extrabold lowercase md:mx-auto md:max-w-328 md:px-10 md:text-sm">
                <span class="min-w-0 flex-1 truncate text-destack-cream">© Symbol Industries</span>

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

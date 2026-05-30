const communityLinks = [
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["twitter", "https://twitter.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

export function Footer() {
    return (
        <footer class="border-t border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between gap-4 px-4 text-sm font-extrabold lowercase md:px-10">
                <span class="hidden text-destack-panel sm:inline">© Symbol Industries</span>

                <nav class="flex items-center gap-2">
                    {communityLinks.map(([label, href], index) => (
                        <>
                            {index > 0 && <Separator />}
                            <a class="hover:text-destack-accent" href={href}>
                                {label}
                            </a>
                        </>
                    ))}
                </nav>

                <a
                    class="hidden hover:text-destack-accent sm:inline"
                    href="https://github.com/destack-sh/destack"
                >
                    <span class="border-b-4 border-destack-panel/40">100%</span> open source
                </a>
            </div>
        </footer>
    );
}

function Separator() {
    return <span class="text-destack-panel/35">·</span>;
}

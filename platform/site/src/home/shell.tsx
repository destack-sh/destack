import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

const communityLinks = [
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["twitter", "https://twitter.com/destack"],
    ["github", "https://github.com/destack-sh/destack"],
] as const;

type PageShellProps = {
    children: JSX.Element;
};

export function PageShell(props: PageShellProps) {
    return (
        <main class="grid min-h-[100svh] min-w-0 grid-rows-[3rem_minmax(0,1fr)_3rem] overflow-x-hidden bg-destack-page font-mono text-neutral-950 lg:h-[100svh] lg:overflow-hidden">
            <TopBar />
            <section class="mx-auto grid h-full min-h-0 min-w-0 w-full max-w-[82rem] grid-rows-[auto_minmax(0,1fr)] gap-6 px-6 pt-7 pb-12">
                {props.children}
            </section>
            <Footer />
        </main>
    );
}

function TopBar() {
    return (
        <header class="border-b border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-[82rem] items-center justify-between px-10">
                <A class="flex items-center gap-2 text-sm font-extrabold" href="/">
                    <span class="grid size-7 place-items-center overflow-hidden rounded-full">
                        <img
                            alt=""
                            class="size-8 max-w-none"
                            height="32"
                            src="/brand/favicon/favicon-simple.svg"
                            width="32"
                        />
                    </span>
                    <span>destack</span>
                </A>

                <nav class="hidden items-center gap-2 text-sm font-extrabold lowercase md:flex">
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

function Footer() {
    return (
        <footer class="border-t border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-[82rem] items-center justify-between px-10 text-sm font-extrabold lowercase">
                <span>destack.sh</span>
                <span>
                    <span class="border-b-4 border-destack-panel/40">100%</span> open source
                </span>
            </div>
        </footer>
    );
}

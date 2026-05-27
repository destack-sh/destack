import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

import { Icon } from "./icon";

const githubUrl = "https://github.com/destack-sh/destack";

const communityLinks = [
    ["discord", "https://discord.gg/xUFQ45TWYd"],
    ["twitter", "https://twitter.com/destack"],
    ["github", githubUrl],
] as const;

type PageShellProps = {
    children: JSX.Element;
};

export function PageShell(props: PageShellProps) {
    return (
        <main class="grid min-h-svh min-w-0 grid-rows-[3rem_minmax(0,1fr)_3rem] overflow-x-hidden bg-destack-page font-mono text-neutral-950 lg:h-svh lg:overflow-hidden">
            {/* top bar */}
            <TopBar />

            {/* page body */}
            <section class="mx-auto grid h-full min-h-0 w-full max-w-328 min-w-0 grid-rows-[auto_minmax(0,1fr)] gap-6 px-6 pt-7 pb-12">
                {props.children}
            </section>

            {/* footer */}
            <Footer />
        </main>
    );
}

function TopBar() {
    return (
        <header class="border-b border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between px-10">
                {/* brand */}
                <A class="flex items-center gap-2 text-sm font-extrabold" href="/">
                    <Icon class="size-7" />
                    <span>destack</span>
                </A>

                {/* community links, separated by slashes */}
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
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between px-10 text-sm font-extrabold lowercase">
                {/* copyright */}
                <span class="text-destack-panel/70">© Symbol Industries</span>

                {/* open source link */}
                <a class="hover:text-destack-accent" href={githubUrl}>
                    <span class="border-b-4 border-destack-panel/40">100%</span> open source
                </a>
            </div>
        </footer>
    );
}

import { A } from "@solidjs/router";

import { Icon } from "./icon";

const productLinks = [
    ["blog", "/blog/"],
    ["fiddle", "/#fiddle"],
    ["install", "/#install"],
] as const;

export function TopBar() {
    return (
        <header class="border-b border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between gap-3 px-4 md:px-10">
                <A class="flex min-w-0 items-center gap-2 text-sm font-extrabold" href="/">
                    <Icon class="size-7" />
                    <span>destack</span>
                </A>

                <nav class="flex shrink-0 items-center gap-1 text-xs font-extrabold lowercase sm:gap-2 sm:text-sm">
                    {productLinks.map(([label, href], index) => (
                        <>
                            {index > 0 && <Separator />}
                            {href.startsWith("/#") ? (
                                <a class="hover:text-destack-accent" href={href}>
                                    {label}
                                </a>
                            ) : (
                                <A class="hover:text-destack-accent" href={href}>
                                    {label}
                                </A>
                            )}
                        </>
                    ))}
                </nav>
            </div>
        </header>
    );
}

function Separator() {
    return <span class="text-destack-panel/35">·</span>;
}

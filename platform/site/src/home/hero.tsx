import type { ParentProps } from "solid-js";

import { Install } from "./install";

export function Hero() {
    return (
        <div class="flex flex-col gap-y-4 md:gap-y-6">
            <h1 class="page-title w-full px-2 md:px-4">
                engineer impeccable software with confidence
            </h1>

            <div class="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-6 px-2 md:px-4 lg:grid-cols-[minmax(0,1fr)_29rem] lg:items-center">
                <section class="flex flex-col gap-y-2 md:gap-y-3">
                    <p class="max-w-3xl text-sm leading-6 font-bold text-neutral-700">
                        <Underline>destack</Underline> is a deeply integrated stack for building
                        correct, optimal, integrated software on one{" "}
                        <Underline>unified, native TypeScript++ toolchain</Underline>, VM,
                        compiler, debugger, and runtime
                    </p>
                </section>

                <Install />
            </div>

            <ul class="flex min-w-0 flex-wrap gap-x-5 gap-y-2 px-2 text-sm font-extrabold lowercase md:px-4">
                <Claim>build correct full-stack systems</Claim>
                <Claim>write strict TypeScript(++) everywhere</Claim>
                <Claim>run natively <span class="italic">and</span> on the web</Claim>
                <Claim>ship software that just works</Claim>
            </ul>
        </div>
    );
}

function Underline(props: ParentProps) {
    return <span class="underline decoration-2 underline-offset-4">{props.children}</span>;
}

function Claim(props: ParentProps) {
    return (
        <li class="flex items-center gap-2">
            <span class="size-2 rounded-full bg-destack-accent" />
            <span>{props.children}</span>
        </li>
    );
}

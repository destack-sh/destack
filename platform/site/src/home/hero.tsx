import { Install } from "./install";

export function Hero() {
    return (
        <div class="flex gap-y-1 md:gap-y-3 flex-col">
            {/* title */}
            <h1 class="max-w-4xl w-full text-3xl px-2 md:px-4 leading-none font-extrabold md:text-3xl">
                build magnificient software that lasts
            </h1>

            {/* main */}
            <div class="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 px-2 md:px-4 lg:grid-cols-[minmax(0,1fr)_29rem] lg:items-center">
                <section class="flex flex-col gap-y-1 md:gap-y-3">
                    {/* tagline */}
                    <p class="max-w-3xl text-sm leading-6 font-bold text-neutral-700">
                        <Underline>destack</Underline> is a fully integrated stack for building
                        correct, optimal, integrated software based on a{" "}
                        <Underline>unified, native TypeScript++ toolchain</Underline>, VM, compiler, linter, and runtime
                    </p>

                </section>

                {/* install */}
                <Install />
            </div>

            {/* "features" */}
            <ul class="px-2 md:px-4 flex min-w-0 flex-wrap gap-x-5 gap-y-2 text-sm font-extrabold lowercase">
                <Claim>engineer precise systems</Claim>
                <Claim>craft software with care</Claim>
                <Claim>ship software confidently</Claim>
                <Claim>sleep well every night</Claim>
                <Claim>leverage proven tech</Claim>
            </ul>
        </div>
    );
}

function Underline(props: { children: string }) {
    return <span class="underline decoration-2 underline-offset-4">{props.children}</span>;
}

function Claim(props: { children: string }) {
    return (
        <li class="flex items-center gap-2">
            <span class="size-2 rounded-full bg-destack-accent" />
            <span>{props.children}</span>
        </li>
    );
}

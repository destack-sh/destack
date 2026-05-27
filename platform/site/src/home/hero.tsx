import { Install } from "./install";

export function Hero() {
    return (
        <div class="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 px-2 md:px-4 py-2 lg:grid-cols-[minmax(0,1fr)_29rem] lg:items-center">
            <section class="min-w-0">
                {/* title */}
                <h1 class="max-w-4xl text-3xl leading-none font-extrabold md:text-3xl">
                    engineer great software with confidence
                </h1>

                {/* tagline */}
                <p class="mt-4 max-w-3xl text-sm leading-6 font-bold text-neutral-700">
                    <Underline>destack</Underline> is a fully integrated stack for building
                    correct, optimal, integrated software based on a{" "}
                    <Underline>unified, native TypeScript++ toolchain</Underline>, VM, compiler, linter, and runtime
                </p>

                {/* "features" */}
                <ul class="mt-3 flex min-w-0 flex-wrap gap-x-5 gap-y-2 text-sm font-extrabold lowercase">
                    <Claim>build precision software</Claim>
                    <Claim>ship with care and pride</Claim>
                    <Claim>sleep well every night</Claim>
                </ul>
            </section>

            <Install />
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

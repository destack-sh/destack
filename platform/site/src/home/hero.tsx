import { createSignal } from "solid-js";

import { DottedFrame } from "./dotted";

const installCommand = "curl -fsSL https://destack.sh/install | sh";

export function Hero() {
    return (
        <div class="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-4 px-4 py-2 lg:grid-cols-[minmax(0,1fr)_29rem] lg:items-center">
            <section class="min-w-0">
                <h1 class="max-w-4xl text-[2.65rem] font-extrabold leading-none md:text-3xl">
                    engineer great software with confidence
                </h1>
                <p class="mt-4 max-w-3xl text-sm font-bold leading-6 text-neutral-700">
                    <span>de•stack</span> is a
                    fully integrated stack for building correct, optimal, integrated software
                    with a full-stack language toolchain, VM, AOT compiler, and deep code analysis.
                </p>
                <ul class="mt-3 flex min-w-0 flex-wrap gap-x-5 gap-y-2 text-sm font-extrabold lowercase">
                    <Claim>deeply understand your systems</Claim>
                    <Claim>ship better software faster</Claim>
                    <Claim>sleep well at night</Claim>
                </ul>
            </section>

            <Install />
        </div>
    );
}

function Claim(props: { children: string }) {
    return (
        <li class="flex items-center gap-2">
            <span class="size-2 rounded-full bg-destack-accent" />
            <span>{props.children}</span>
        </li>
    );
}

function Install() {
    return (
        <section class="flex min-w-0 items-center lg:justify-end">
            <DottedFrame class="w-full max-w-[30rem]" depth="small">
                <InstallCommand />
            </DottedFrame>
        </section>
    );
}

function InstallCommand() {
    const [isCopied, setIsCopied] = createSignal(false);

    const copyInstallCommand = async () => {
        await navigator.clipboard.writeText(installCommand);
        setIsCopied(true);
        window.setTimeout(() => setIsCopied(false), 1200);
    };

    return (
        <div class="relative min-w-0 border-[2.5px] border-neutral-950 bg-destack-panel px-3 py-3">
            <span class="absolute -top-3 left-3 bg-destack-page px-1 text-sm font-extrabold lowercase">
                install
            </span>
            <div class="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3">
                <span class="text-sm font-extrabold text-destack-accent">$</span>
                <code class="min-w-0 overflow-x-auto whitespace-nowrap text-sm font-extrabold leading-5">
                    {installCommand}
                </code>
                <button
                    class="border-b-4 border-neutral-300 text-sm font-extrabold lowercase hover:border-destack-accent"
                    onClick={copyInstallCommand}
                    type="button"
                >
                    {isCopied() ? "copied" : "copy"}
                </button>
            </div>
        </div>
    );
}

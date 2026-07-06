import { A } from "@solidjs/router";
import { createSignal, onCleanup } from "solid-js";

import { Icon } from "./icon";

const copiedFeedbackMs = 2000;
const installCommand = "curl -fsSL https://destack.sh/install | sh";

export function TopBar() {
    return (
        <header class="border-b border-neutral-950 bg-destack-ink text-destack-cream">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-between gap-3 px-4 md:px-10">
                <A class="flex min-w-0 items-center gap-2 text-sm font-extrabold" href="/">
                    <Icon class="size-7" />
                    <span>destack</span>
                </A>

                <InstallCommand />
            </div>
        </header>
    );
}

function InstallCommand() {
    const [isCopied, setIsCopied] = createSignal(false);
    let timer: ReturnType<typeof setTimeout> | undefined;

    const copy = async () => {
        await navigator.clipboard.writeText(installCommand);

        clearTimeout(timer);
        setIsCopied(true);
        timer = setTimeout(() => setIsCopied(false), copiedFeedbackMs);
    };

    onCleanup(() => clearTimeout(timer));

    return (
        <button
            aria-label="copy install command"
            class="group flex min-w-0 max-w-[calc(100vw-8rem)] items-baseline gap-2 text-xs font-extrabold whitespace-nowrap"
            onClick={copy}
            type="button"
        >
            <span class="text-destack-accent">$</span>
            <code class="min-w-0 truncate">{installCommand}</code>
            <span class="hidden w-[6ch] text-left text-destack-cream/50 lowercase group-hover:text-destack-accent sm:block">
                {isCopied() ? "copied" : "copy"}
            </span>
        </button>
    );
}

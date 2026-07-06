import { A } from "@solidjs/router";
import { createSignal, onCleanup } from "solid-js";

import { Icon } from "./icon";

const copiedFeedbackMs = 2000;
const installCommand = "curl -fsSL https://destack.sh/install | sh";

export function TopBar() {
    return (
        <header class="border-b border-destack-cream/20 bg-destack-ink text-destack-cream">
            <div class="flex h-14 max-w-[22rem] items-center justify-between gap-3 overflow-hidden px-4 md:mx-auto md:max-w-328 md:px-10">
                <A class="flex shrink-0 items-center gap-2 text-sm font-extrabold" href="/">
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
            class="group flex min-w-0 max-w-[10rem] items-baseline gap-2 text-xs font-extrabold whitespace-nowrap sm:max-w-[calc(100vw-8rem)]"
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

import { createSignal, onCleanup } from "solid-js";

const copiedFeedbackMs = 2000;
const installCommand = "curl -fsSL https://destack.sh/install | sh";

export function Footer() {
    return (
        <footer class="border-t border-neutral-950 bg-destack-ink text-destack-cream">
            <div class="mx-auto grid h-12 max-w-328 grid-cols-[1fr_auto_1fr] items-center gap-4 px-4 text-sm font-extrabold lowercase md:px-10">
                <span class="hidden truncate text-destack-cream sm:inline">
                    © Symbol Industries
                </span>

                <InstallCommand />

                <a
                    class="hidden justify-self-end hover:text-destack-accent sm:inline"
                    href="https://github.com/destack-sh/destack"
                >
                    <span class="border-b-4 border-destack-cream/40">100%</span> open source
                </a>
            </div>
        </footer>
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
            class="group flex min-w-0 items-baseline gap-2 text-xs font-extrabold whitespace-nowrap"
            onClick={copy}
            type="button"
        >
            <span class="text-destack-accent">$</span>
            <code class="min-w-0 truncate">{installCommand}</code>
            <span class="w-[6ch] text-left text-destack-cream/50 lowercase group-hover:text-destack-accent">
                {isCopied() ? "copied" : "copy"}
            </span>
        </button>
    );
}

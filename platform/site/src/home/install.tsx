import { createSignal, onCleanup } from "solid-js";

import { Panel } from "../component/panel";

const copiedFeedbackMs = 2000;
const installCommand = "curl -fsSL https://destack.sh/install | sh";

export function Install() {
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
        <section id="install" class="flex min-w-0 items-center lg:justify-end">
            <Panel class="w-full max-w-120" depth="shallow" title="install">
                <div class="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-baseline gap-4 px-4 pt-4.5 pb-3">
                    <span class="text-base leading-5 font-black text-destack-accent">$</span>

                    <code class="min-w-0 overflow-x-auto text-sm leading-5 font-extrabold whitespace-nowrap">
                        {installCommand}
                    </code>

                    <button
                        class="border-b-4 border-neutral-300 text-sm leading-5 font-extrabold lowercase hover:border-destack-accent"
                        onClick={copy}
                        type="button"
                    >
                        {isCopied() ? "copied" : "copy"}
                    </button>
                </div>
            </Panel>
        </section>
    );
}

import { createSignal } from "solid-js";

import { Panel } from "./panel";

// how long the "copied" feedback flashes after a copy
const COPIED_FEEDBACK_MS = 2000;

export function Install() {
    const [isCopied, setIsCopied] = createSignal(false);

    // copy the command to the clipboard, then flash "copied" briefly
    const copy = async () => {
        await navigator.clipboard.writeText("curl -fsSL https://destack.sh/install | sh");
        setIsCopied(true);
        window.setTimeout(() => setIsCopied(false), COPIED_FEEDBACK_MS);
    };

    return (
        <section class="flex min-w-0 items-center lg:justify-end">
            <Panel class="w-full max-w-120" depth="shallow" title="install">
                <div class="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-baseline gap-3 px-3 pt-3 pb-2.5">
                    {/* prompt */}
                    <span class="text-base leading-5 font-black text-destack-accent">$</span>

                    {/* command */}
                    <code class="min-w-0 overflow-x-auto text-sm leading-5 font-extrabold whitespace-nowrap">
                        curl -fsSL https://destack.sh/install | sh
                    </code>

                    {/* copy button */}
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

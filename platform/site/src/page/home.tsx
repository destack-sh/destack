import { Seo } from "../component/seo";
import { Shell } from "../component/shell";

export function HomePage() {
    return (
        <Shell>
            <Seo />

            <section class="bg-destack-ink text-destack-cream">
                <div class="mx-auto flex min-h-[calc(100svh-6rem)] w-full max-w-328 flex-col justify-center gap-8 px-4 py-10 md:px-10">
                    <header class="flex flex-col">
                        <h1 class="max-w-5xl text-4xl leading-none font-black md:text-5xl">
                            engineer impeccable software
                        </h1>
                    </header>

                    <div class="flex flex-col gap-8 lg:flex-row lg:items-start">
                        <div class="flex min-w-0 flex-col lg:w-1/2">
                            <ul class="flex max-w-3xl list-disc flex-col gap-2 pl-5 text-base leading-7 font-bold text-destack-cream marker:text-destack-accent">
                                <li>complete open source "TypeScript++" toolchain</li>
                                <li>statically compilable TypeScript with Rust-y extensions</li>
                                <li>target web & native with the same code (including TSX)</li>
                                <li>introspectable VM, modern AOT compiler</li>
                                <li>integrated runtime with "free" builtin DST</li>
                                <li>absurdly integrated stack for correct software systems</li>
                            </ul>
                        </div>

                        <pre class="min-w-0 overflow-x-auto text-[0.62rem] leading-[1.05rem] font-black text-destack-cream md:text-xs md:leading-5 lg:w-1/2">
                        <code>{`                      ╔═ destack ═══════════════════════════════════╗
╔════════════╗        ║                                             ║░
║            ║░       ║  ┏━━━━━━━━━━━━┓   ┏━━━━━━━━━━━━━━━━━━━━━━┓  ║░
║  source    ║░──────▶║  ┃ language   ┃──▶┃ compiler + runtime   ┃  ║░
║  ts / ds   ║░       ║  ┗━━━━━━━━━━━━┛   ┗━━━━━━━━━━━━━━━━━━━━━━┛  ║░
╚════════════╝░       ║          │                    │             ║░
 ░░░░░░░░░░░░░░       ║          ▼                    ▼             ║░
                      ║  ┏━━━━━━━━━━━━┓   ┏━━━━━━━━━━━━━━━━━━━━━━┓  ║░
                      ║  ┃ stdlib     ┃──▶┃ native / web targets  ┃  ║░
                      ║  ┗━━━━━━━━━━━━┛   ┗━━━━━━━━━━━━━━━━━━━━━━┛  ║░
                      ║                                             ║░
                      ╚═════════════════════════════════════════════╝░
                       ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░`}</code>
                        </pre>
                    </div>
                </div>
            </section>
        </Shell>
    );
}

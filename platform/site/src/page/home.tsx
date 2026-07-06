import { Seo } from "../component/seo";
import { Shell } from "../component/shell";

export function HomePage() {
    return (
        <Shell>
            <Seo />

            <section class="bg-white text-destack-text">
                <div class="mx-auto flex min-h-[calc(100svh-7rem)] w-full max-w-328 flex-col gap-8 px-4 pt-12 pb-10 md:px-10 md:pt-28 lg:pt-36">
                    <header class="flex flex-col">
                        <h1 class="max-w-[22rem] text-[2rem] leading-none font-black md:max-w-5xl md:text-5xl">
                            engineer impeccable software
                        </h1>
                    </header>

                    <div class="flex flex-col gap-8 lg:flex-row lg:items-start">
                        <div class="flex max-w-[22rem] min-w-0 flex-col md:max-w-none lg:w-[40%]">
                            <ul class="flex max-w-[22rem] list-disc flex-col gap-2 pl-5 text-sm leading-6 font-bold text-destack-text marker:text-destack-accent md:max-w-3xl md:text-base md:leading-7">
                                <li>complete open source "TypeScript++" toolchain</li>
                                <li>statically compilable TypeScript with Rust-y extensions</li>
                                <li>target web & native with the same code (including TSX)</li>
                                <li>introspectable VM, modern AOT compiler</li>
                                <li>integrated runtime with "free" builtin DST</li>
                                <li>absurdly integrated stack for correct software systems</li>
                            </ul>
                        </div>

                        <div class="w-full max-w-[22rem] min-w-0 md:max-w-none lg:w-[60%]">
                            <PlaceholderBox />
                        </div>
                    </div>
                </div>
            </section>
        </Shell>
    );
}

function PlaceholderBox() {
    return (
        <svg
            aria-label="placeholder"
            class="block aspect-[16/9] w-full text-destack-frame"
            role="img"
            viewBox="0 0 640 360"
        >
            <path
                d="M35 31C125 24 224 34 319 28C417 23 526 33 606 30C614 98 608 168 612 246C615 299 607 329 596 332C491 337 383 328 278 334C173 341 91 332 34 333C27 244 33 156 29 83C28 54 31 39 35 31Z"
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="5"
            />
            <path
                d="M44 45C152 95 236 146 318 182C410 224 493 282 598 321"
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-width="5"
            />
            <path
                d="M600 43C484 92 405 141 323 181C233 226 143 274 43 322"
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-width="5"
            />
        </svg>
    );
}

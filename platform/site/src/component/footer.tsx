export function Footer() {
    return (
        <footer class="border-t border-neutral-950 bg-destack-ink text-destack-cream">
            <div class="mx-auto grid h-12 max-w-328 grid-cols-[1fr_auto_1fr] items-center gap-4 px-4 text-sm font-extrabold lowercase md:px-10">
                <span class="hidden truncate text-destack-cream sm:inline">
                    © Symbol Industries
                </span>

                <p class="hidden min-w-0 truncate text-sm font-extrabold lowercase sm:block">
                    program the universe
                </p>

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


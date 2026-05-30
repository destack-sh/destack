const githubUrl = "https://github.com/destack-sh/destack";

export function Footer() {
    return (
        <footer class="border-t border-neutral-950 bg-destack-ink text-destack-panel">
            <div class="mx-auto flex h-12 max-w-328 items-center justify-end gap-4 px-4 text-sm font-extrabold lowercase sm:justify-between md:px-10">
                <span class="hidden text-destack-panel sm:inline">© Symbol Industries</span>

                <a class="hover:text-destack-accent" href={githubUrl}>
                    <span class="border-b-4 border-destack-panel/40">100%</span> open source
                </a>
            </div>
        </footer>
    );
}

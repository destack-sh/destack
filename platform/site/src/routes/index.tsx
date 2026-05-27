import { Fiddle } from "../home/fiddle";
import { Footer } from "../home/footer";
import { Hero } from "../home/hero";
import { TopBar } from "../home/topbar";

export default function Index() {
    return (
        <main class="grid min-h-svh min-w-0 grid-rows-[3rem_minmax(0,1fr)_3rem] overflow-x-hidden bg-destack-page font-mono text-neutral-950 lg:h-svh lg:overflow-hidden">
            <TopBar />

            {/* page body */}
            <section class="mx-auto grid h-full min-h-0 w-full max-w-328 min-w-0 grid-rows-[auto_minmax(0,1fr)] gap-4 px-4 pt-6 pb-8 md:gap-6 md:px-6 md:pt-7 md:pb-12">
                <Hero />
                <Fiddle />
            </section>

            <Footer />
        </main>
    );
}

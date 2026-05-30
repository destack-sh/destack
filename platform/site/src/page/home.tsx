import { Shell } from "../component/shell";
import { Seo } from "../component/seo";
import { Fiddle } from "../home/fiddle";
import { Hero } from "../home/hero";

export function HomePage() {
    return (
        <Shell isFixed>
            <Seo />
            <section class="mx-auto grid h-full min-h-0 w-full max-w-328 min-w-0 grid-rows-[auto_minmax(0,1fr)] gap-7 px-4 pt-8 pb-10 md:gap-10 md:px-6 md:pt-12 md:pb-16">
                <Hero />

                <div id="fiddle" class="min-h-0">
                    <Fiddle />
                </div>
            </section>
        </Shell>
    );
}

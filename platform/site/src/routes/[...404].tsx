import { A } from "@solidjs/router";
import { HttpStatusCode } from "@solidjs/start";

import { Seo } from "../component/seo";
import { Shell } from "../component/shell";

export default function NotFound() {
    return (
        <Shell>
            <HttpStatusCode code={404} />
            <Seo title="404" description="This page does not exist." />

            <section class="mx-auto grid w-full max-w-328 gap-4 px-4 py-16 md:px-10">
                <p class="text-sm font-extrabold text-destack-accent lowercase">404</p>
                <h1 class="page-title">this page does not exist</h1>
                <p class="max-w-2xl text-sm leading-6 font-bold text-destack-soft">
                    the address may have moved, or it never existed in the first place.
                </p>
                <A
                    class="w-max border-b-4 border-destack-line text-sm font-extrabold lowercase hover:border-destack-accent"
                    href="/"
                >
                    back to destack
                </A>
            </section>
        </Shell>
    );
}

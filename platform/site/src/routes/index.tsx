import { Fiddle } from "../home/fiddle";
import { Hero } from "../home/hero";
import { PageShell } from "../home/shell";

export default function Index() {
    return (
        <PageShell>
            <Hero />
            <Fiddle />
        </PageShell>
    );
}

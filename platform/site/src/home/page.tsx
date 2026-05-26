import { Fiddle } from "./fiddle";
import { Hero } from "./hero";
import { PageShell } from "./shell";

export default function Home() {
    return (
        <PageShell>
            <Hero />
            <Fiddle />
        </PageShell>
    );
}

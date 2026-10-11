import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { Metadata } from "@destack/view/document";

import { Adjective } from "../home/adjective/adjective";
import { Noun } from "../home/noun/noun";
import { BottomBar } from "../home/entry/switch";
import { Imperative } from "../home/imperative/imperative";
import { Participle } from "../home/participle/participle";
import { firstPicks, promptOf } from "../home/participle/wall";
import { PastParticiple } from "../home/past/past";
import { Verb } from "../home/verb/verb";
import { lattice } from "../layout/lattice.stylex";
import "../home/figure/figure.css";

/** Render the public Destack homepage: the word in its forms, from verb to participle, closing on the verb as an instruction. */
export function Home() {
    // hold whether the stack is open, which every figure follows
    const [isOpen, setIsOpen] = createSignal(false);

    // hold the tools the reader uses, starting from the first figure's, which the install prompt names
    const [picks, setPicks] = createSignal<readonly string[]>(firstPicks);
    const pick = (name: string) =>
        setPicks((current) =>
            current.includes(name)
                ? current.filter((picked) => picked !== name)
                : [...current, name],
        );

    return (
        <>
            <Metadata alternates={{ canonical: "/" }} />
            <Verb isOpen={isOpen()} />
            <PastParticiple isOpen={isOpen()} onChange={setIsOpen} />
            <Noun isOpen={isOpen()} />
            <Adjective isOpen={isOpen()} />
            <Participle isOpen={isOpen()} picks={picks()} onPick={pick} />
            <Imperative prompt={promptOf(picks())} />
            <BottomBar isOpen={isOpen()} />
            <Interval />
        </>
    );
}

/** Space the last section apart from the footer inside the frame, so its edges run on to it. */
function Interval() {
    return <div aria-hidden="true" {...style.attrs(lattice.frame, lattice.interval)} />;
}

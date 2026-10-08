import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { Metadata } from "@destack/view/document";

import { Adjective } from "../home/adjective/adjective";
import { Noun } from "../home/noun/noun";
import { Participle } from "../home/participle/participle";
import { PastParticiple } from "../home/past/past";
import { Verb } from "../home/verb/verb";
import { lattice } from "../layout/lattice.stylex";
import "../home/figure/figure.css";

/** Render the public Destack homepage: the word in its forms, from verb to participle. */
export function Home() {
    // hold whether the stack is open, which every figure follows
    const [isOpen, setIsOpen] = createSignal(false);

    return (
        <>
            <Metadata alternates={{ canonical: "/" }} />
            <Verb isOpen={isOpen()} />
            <PastParticiple isOpen={isOpen()} onChange={setIsOpen} />
            <Noun isOpen={isOpen()} />
            <Adjective isOpen={isOpen()} />
            <Participle isOpen={isOpen()} />
            <Interval />
        </>
    );
}

/** Space the last section apart from the footer inside the frame, so its edges run on to it. */
function Interval() {
    return <div aria-hidden="true" {...style.attrs(lattice.frame, lattice.interval)} />;
}

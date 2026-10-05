import * as stylex from "@destack/style";
import { createSignal } from "@destack/view";

import { Adjective } from "../home/adjective";
import { Noun } from "../home/noun";
import { Participle } from "../home/participle";
import { Verb } from "../home/verb";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { lattice } from "../style/lattice.stylex";

/** Render the public Destack homepage: the word in its forms, from verb to participle. */
export function HomePage() {
    // hold whether the stack is open, which every figure follows
    const [isOpen, setIsOpen] = createSignal(false);

    return (
        <Shell>
            <Seo />
            <Verb isOpen={isOpen()} onChange={setIsOpen} />
            <Noun isOpen={isOpen()} />
            <Adjective isOpen={isOpen()} />
            <Participle />
            <Interval />
        </Shell>
    );
}

/** Space the last section apart from the footer inside the frame, so its edges run on to it. */
function Interval() {
    return <div aria-hidden="true" {...stylex.attrs(lattice.frame, lattice.interval)} />;
}

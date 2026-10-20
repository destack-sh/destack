import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { Metadata } from "@destack/view/document";

import { Noun } from "../home/noun/noun";
import { BottomBar } from "../home/entry/switch";
import { Adjective } from "../home/adjective/adjective";
import { Command } from "../home/command/command";
import { Participle } from "../home/participle/participle";
import { firstPicks, promptOf } from "../home/participle/wall";
import type { Tool } from "../home/tool/tool";
import { PastParticiple } from "../home/past/past";
import { Verb } from "../home/verb/verb";
import { lattice } from "../layout/lattice.stylex";
import "../home/figure/figure.css";

/** Render the public Destack homepage. */
export function Home() {
    // hold whether the stack is open, which every figure follows
    const [isOpen, setIsOpen] = createSignal(false);

    // hold the tools the reader uses, starting from the first figure's, which the install prompt names
    const [picks, setPicks] = createSignal<readonly Tool[]>(firstPicks);
    const pick = (tool: Tool) =>
        setPicks((current) =>
            current.includes(tool)
                ? current.filter((picked) => picked !== tool)
                : [...current, tool],
        );

    // choose a tool from the first figure: pick it, last, so that figure shows it
    const choose = (tool: Tool) =>
        setPicks((current) => [...current.filter((picked) => picked !== tool), tool]);

    return (
        <>
            <Metadata alternates={{ canonical: "/" }} />
            <Verb isOpen={isOpen()} picks={picks()} onChoose={choose} />
            <PastParticiple isOpen={isOpen()} onChange={setIsOpen} />
            <Noun isOpen={isOpen()} />
            <Command isOpen={isOpen()} />
            <Adjective isOpen={isOpen()} />
            <Participle
                isOpen={isOpen()}
                picks={picks()}
                onPick={pick}
                prompt={promptOf(picks())}
            />
            <BottomBar isOpen={isOpen()} />
            <Interval />
        </>
    );
}

/** Space the last section apart from the footer inside the frame, so its edges run on to it. */
function Interval() {
    return <div aria-hidden="true" {...style.attrs(lattice.frame, lattice.interval)} />;
}

import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";

import { lattice } from "../../layout/lattice.stylex";
import { FigureLabel } from "../entry/entry";
import { LabelRow, pane, PaneLabel } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { Bill } from "../figure/bill";
import { Dial, teams } from "../figure/dial";
import { BASE_USERS } from "../figure/price";
import { createSteps } from "../figure/stagger";
import type { Tool } from "../tool/tool";
import { lineCount, tallyOf, ToolWall } from "./wall";

/** The panes the figure switches between below the desktop width, the tools and their bill. */
const PANES = ["Your tools", "Your bill"];

/** Draw the tools you pick beside their bill, rented per seat or paid by usage, as a figure. */
export function Tally(properties: {
    number: number;
    stacked: string;
    destacked: string;
    isOpen: boolean;
    picks: readonly Tool[];
    onPick: (tool: Tool) => void;
}) {
    // follow the motion the figure shows, and step the bill's lines in order as the stack switches
    const [stage, setStage] = createSignal<HTMLElement>();
    const isReduced = createReducedMotion(stage);
    const { isOpenAt } = createSteps(() => properties.isOpen, lineCount, isReduced);

    // read the bill for a team of the size the reader dials
    const [people, setPeople] = createSignal<number>(5);

    // show the tools or their bill below the desktop width, where one pane shows at a time
    const [shownPane, setShownPane] = createSignal(0);

    return (
        <>
            <FigureLabel
                number={properties.number}
                isOpen={properties.isOpen}
                stacked={properties.stacked}
                destacked={properties.destacked}
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div
                ref={setStage}
                {...style.attrs(
                    lattice.cell,
                    lattice.figureCell,
                    styles.stage,
                    shownPane() !== 0 && pane.hidden,
                )}
            >
                <LabelRow end={undefined}>
                    <PaneLabel panes={PANES} shown={shownPane()} onShow={setShownPane}>
                        {properties.isOpen ? "Your tools, owned" : "Your tools, rented"}
                    </PaneLabel>
                </LabelRow>
                <ToolWall
                    picks={properties.picks}
                    isOpen={properties.isOpen}
                    onPick={properties.onPick}
                />
            </div>
            <div
                {...style.attrs(
                    lattice.cell,
                    lattice.figureCell,
                    styles.key,
                    shownPane() !== 1 && pane.hidden,
                )}
            >
                <LabelRow
                    end={
                        <Dial
                            label="People"
                            options={teams}
                            value={people()}
                            format={String}
                            onChange={setPeople}
                        />
                    }
                >
                    <PaneLabel panes={PANES} shown={shownPane()} onShow={setShownPane}>
                        {properties.isOpen ? "Owned, paid by usage" : "Rented, paid per seat"}
                    </PaneLabel>
                </LabelRow>
                <Bill
                    isWindow
                    lines={tallyOf(properties.picks)}
                    scale={{ people: people(), users: BASE_USERS }}
                    scales={teams.map((team) => ({ people: team, users: BASE_USERS }))}
                    isOpenAt={isOpenAt}
                />
            </div>
        </>
    );
}

/** The tally styles. */
const styles = style.create({
    stage: {
        gridColumn: { default: "1 / 8", [media.maxLg]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    key: {
        gridColumn: { default: "8 / -1", [media.maxLg]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
});

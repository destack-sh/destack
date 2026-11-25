import type { JSX } from "@destack/view";
import { MenuAction } from "../menu/index.ts";
import { closePane, evenPanes, maximizePane, moveTabTo, splitDown, splitRight } from "./actions.ts";

/** Render what a menu offers for a pane: splitting, moving, covering and closing, where it can. */
export function PaneMenuItems(): JSX.Element {
    return (
        <>
            <MenuAction action={splitRight} />
            <MenuAction action={splitDown} />
            <MenuAction action={moveTabTo} />
            <MenuAction action={maximizePane} />
            <MenuAction action={evenPanes} />
            <MenuAction action={closePane} />
        </>
    );
}

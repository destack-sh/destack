import type { JSX } from "@destack/view";
import { MenuAction } from "../menu/index.ts";
import {
    closePane,
    dockPane,
    evenPanes,
    floatTab,
    maximizePane,
    moveTabTo,
    openWindow,
    splitDown,
    splitRight,
} from "./actions.ts";

/** Render what a menu offers for a pane: splitting, moving, covering and closing, where it can. */
export function PaneMenuItems(): JSX.Element {
    return (
        <>
            <MenuAction action={splitRight} />
            <MenuAction action={splitDown} />
            <MenuAction action={moveTabTo} />
            <MenuAction action={floatTab} />
            <MenuAction action={dockPane} />
            <MenuAction action={openWindow} />
            <MenuAction action={maximizePane} />
            <MenuAction action={evenPanes} />
            <MenuAction action={closePane} />
        </>
    );
}

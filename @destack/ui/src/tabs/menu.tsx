import type { JSX } from "@destack/view";
import { MenuAction } from "../menu/index.ts";
import {
    closeAllTabs,
    closeOtherTabs,
    closeTab,
    closeTabsAfter,
    moveTabLeft,
    moveTabRight,
    reopenTab,
} from "./actions.ts";

/** Render what a menu offers for a tab: closing it and others, and moving it, where each can. */
export function TabsMenuItems(): JSX.Element {
    return (
        <>
            <MenuAction action={closeTab} />
            <MenuAction action={closeOtherTabs} />
            <MenuAction action={closeTabsAfter} />
            <MenuAction action={closeAllTabs} />
            <MenuAction action={reopenTab} />
            <MenuAction action={moveTabLeft} />
            <MenuAction action={moveTabRight} />
        </>
    );
}

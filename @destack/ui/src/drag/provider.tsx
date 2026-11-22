import { createContext, type JSX, useContext, useEnvironment, useLocale } from "@destack/view";
import { DragAnnouncer } from "./announcer.tsx";
import { DragManager } from "./drag.ts";

/** The drags shared by the components under a provider, null outside one. */
export const DragContext = createContext<DragManager | null>(null);

/** Share one manager of drags with every component under it, so items move between them. */
export function DragProvider(properties: {
    /** The components whose items drag between each other. */
    readonly children?: JSX.Element;
}): JSX.Element {
    // open the page's drags and announce them once
    const manager = new DragManager(useLocale(), useEnvironment());

    return (
        <DragContext value={manager}>
            {properties.children}
            <DragAnnouncer manager={manager} />
        </DragContext>
    );
}

/** Read the drags a component takes part in: the nearest provider's, else a new manager. */
export function useDragManager(): { readonly manager: DragManager; readonly isShared: boolean } {
    const shared = useContext(DragContext);

    return shared === null
        ? { manager: new DragManager(useLocale(), useEnvironment()), isShared: false }
        : { manager: shared, isShared: true };
}

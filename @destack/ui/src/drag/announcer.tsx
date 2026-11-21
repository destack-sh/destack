import { t } from "@destack/locale";
import * as style from "@destack/style";
import { type JSX, useLocale } from "@destack/view";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";
import type { DragManager } from "./drag.ts";

/** Render a drag manager's instructions and the live region that announces each step. */
export function DragAnnouncer(properties: { readonly manager: DragManager }): JSX.Element {
    const locale = useLocale();

    return (
        <>
            {/* Instructions every draggable points to */}
            <div id={properties.manager.instructionsId} hidden>
                {locale.render(
                    t`Press Space to pick up an item. Use the arrow keys to move it, Space to drop it and Escape to cancel.`,
                )}
            </div>

            {/* Announcements */}
            <div
                role="status"
                aria-live="assertive"
                aria-atomic="true"
                data-slot="drag-announcer"
                {...style.attrs(visuallyHiddenStyle())}
            >
                {properties.manager.announcement()}
            </div>
        </>
    );
}

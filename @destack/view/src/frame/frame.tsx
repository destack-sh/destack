import * as style from "@destack/style";
import type { JSX } from "../solid/component.ts";
import { ViewAddress } from "../declare/address.ts";
import { createEffect } from "../solid/reactive.ts";

/** The styles of a framed view: it fills its box, without the frame's own border. */
const styles = style.create({
    frame: {
        blockSize: "100%",
        borderWidth: 0,
        display: "block",
        inlineSize: "100%",
    },
});

/** Frame an installation's view on its own origin, opened through the page's open path. */
export function Frame(properties: {
    /** The view to open. */
    readonly address: ViewAddress;
    /** The revision the installation serves, whose change opens the view again. */
    readonly revision?: string | undefined;
    /** The frame's accessible name, such as the installation's title. */
    readonly title: string;
    /** Styles beside the frame's own. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    // open the view again at each new revision the installation serves
    let frame: HTMLIFrameElement | undefined;
    let opened = properties.revision;
    createEffect(
        () => properties.revision,
        (revision) => {
            if (frame !== undefined && revision !== opened) {
                opened = revision;
                frame.src = ViewAddress.openPath(properties.address);
            }
        },
    );

    return (
        <iframe
            ref={frame}
            src={ViewAddress.openPath(properties.address)}
            title={properties.title}
            allow="clipboard-read; clipboard-write; fullscreen"
            {...style.attrs([styles.frame, properties.xstyle])}
        />
    );
}

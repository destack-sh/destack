import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { space } from "@destack/theme/tokens.stylex";
import { fade } from "./fade.stylex.ts";

/** The fade of what scrolls at each side that `Scroll.follow` marks with more past it. */
export const scrollFade = style.create({
    sides: {
        [fade.top]: { default: "0px", ":is([data-scrollable~=top])": space[6] },
        [fade.bottom]: { default: "0px", ":is([data-scrollable~=bottom])": space[6] },
        [fade.left]: { default: "0px", ":is([data-scrollable~=left])": space[6] },
        [fade.right]: { default: "0px", ":is([data-scrollable~=right])": space[6] },
        maskImage: {
            default: `linear-gradient(to bottom, transparent, black ${fade.top}, black calc(100% - ${fade.bottom}), transparent), linear-gradient(to right, transparent, black ${fade.left}, black calc(100% - ${fade.right}), transparent)`,
            [media.forcedColors]: "none",
        },
        maskComposite: "intersect",
    },
});

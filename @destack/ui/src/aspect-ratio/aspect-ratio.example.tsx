import type { JSX } from "@solidjs/web";
import { AspectRatio } from "./aspect-ratio.tsx";

/** Show a note's cover photo at 16 by 9 whatever the width. */
export function AspectRatioExample(): JSX.Element {
    return (
        <AspectRatio ratio={16 / 9}>
            <img src="/photos/alfama.jpg" alt="Alfama at dusk" width="100%" height="100%" />
        </AspectRatio>
    );
}

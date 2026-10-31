import * as style from "@destack/style";
import { createEffect, createSignal } from "@destack/view";
import type { ImageMetadata } from "@destack/view/media";
import { createReducedMotion } from "@destack/view/motion";

/** Play a video as a silent looping animation, held on its poster frame while motion is reduced. */
export function Animation(properties: {
    source: string;
    alt: string;
    poster: ImageMetadata | undefined;
}) {
    // play while motion runs, and hold the poster frame while it is reduced
    const [video, setVideo] = createSignal<HTMLVideoElement>();
    const isStill = createReducedMotion(video);
    createEffect(
        () => ({ element: video(), still: isStill() }),
        ({ element, still }) => {
            if (element === undefined) {
                return;
            }
            if (still) {
                element.pause();
            } else {
                void element.play();
            }
        },
    );

    return (
        <video
            ref={setVideo}
            muted
            loop
            playsinline
            preload="metadata"
            aria-label={properties.alt}
            src={properties.source}
            poster={properties.poster?.src}
            width={properties.poster?.width}
            height={properties.poster?.height}
            {...style.attrs(styles.media)}
        />
    );
}

/** The animation's styles. */
const styles = style.create({
    media: {
        display: "block",
        width: "100%",
    },
});

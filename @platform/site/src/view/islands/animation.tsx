import type { ImageMetadata } from "@destack/view/media";
import { Animation } from "../reader/animation";

/** Play a video as a silent looping animation, held on its poster frame while motion is reduced. */
export default function AnimationIsland(properties: {
    readonly source: string;
    readonly alt: string;
    readonly poster?: ImageMetadata | undefined;
}) {
    return <Animation source={properties.source} alt={properties.alt} poster={properties.poster} />;
}

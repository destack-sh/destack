import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { radius, space } from "@destack/theme/tokens.stylex";
import { For } from "@destack/view";
import { Cluster } from "../cluster/index.ts";
import { PICTURES } from "../example/picture.ts";
import {
    Carousel,
    CarouselContent,
    CarouselDots,
    CarouselItem,
    CarouselNext,
    CarouselPlay,
    CarouselPrevious,
} from "./carousel.tsx";

/** The photos of a note about a trip. */
const PHOTOS = [
    { url: PICTURES.alfama, caption: "Alfama at dusk" },
    { url: PICTURES.tram, caption: "Tram 28 on the hill" },
    { url: PICTURES.belem, caption: "Pastries in Belém" },
];

/** The styles of a photo, at 16 by 9, and of a carousel with room for its buttons. */
const styles = style.create({
    carousel: {
        width: `min(32rem, 100% - 2 * ${space[7]})`,
        marginInline: "auto",
    },
    controls: {
        marginBlockStart: space[3],
    },
    photo: {
        display: "block",
        width: "100%",
        aspectRatio: "16 / 9",
        borderRadius: radius[4],
        objectFit: "cover",
    },
});

/** The photos of a note one at a time. */
export const carouselTripPhotos = defineExample({
    of: Carousel,
    name: "tripPhotos",
    description: t`The photos of a note one at a time.`,
    properties: {
        "aria-label": "Photos",
    },
    render: (properties) => (
        <Carousel {...properties} xstyle={styles.carousel}>
            <CarouselContent>
                <For each={PHOTOS}>
                    {(photo) => (
                        <CarouselItem>
                            <img
                                src={photo.url}
                                alt={photo.caption}
                                {...style.attrs(styles.photo)}
                            />
                        </CarouselItem>
                    )}
                </For>
            </CarouselContent>
            <CarouselPrevious />
            <CarouselNext />
        </Carousel>
    ),
});

/** Trip photos that rotate on their own, with a button that stops them and a dot per photo. */
export const carouselTripPhotosRotating = defineExample({
    of: Carousel,
    name: "tripPhotosRotating",
    description: t`Trip photos that rotate on their own, with a button that stops them and a dot per photo.`,
    properties: {
        "aria-label": "Trip photos",
        loop: true,
        autoplay: 5000,
    },
    render: (properties) => (
        <Carousel {...properties} xstyle={styles.carousel}>
            <CarouselContent>
                <For each={PHOTOS}>
                    {(photo) => (
                        <CarouselItem>
                            <img
                                src={photo.url}
                                alt={photo.caption}
                                {...style.attrs(styles.photo)}
                            />
                        </CarouselItem>
                    )}
                </For>
            </CarouselContent>
            <Cluster space="3" justify="center" xstyle={styles.controls}>
                <CarouselPlay />
                <CarouselDots />
            </Cluster>
        </Carousel>
    ),
});

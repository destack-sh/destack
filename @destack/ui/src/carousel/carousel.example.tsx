import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { radius } from "@destack/theme/tokens.stylex";
import { For } from "@destack/view";
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

/** The styles of a photo: the width of its slide, at 16 by 9. */
const styles = style.create({
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
    name: "trip-photos",
    description: "the photos of a note one at a time",
    properties: {
        "aria-label": "Photos",
    },
    render: (properties) => (
        <Carousel {...properties}>
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
    name: "trip-photos-rotating",
    description:
        "trip photos that rotate on their own, with a button that stops them and a dot per photo",
    properties: {
        "aria-label": "Trip photos",
        loop: true,
        autoplay: 5000,
    },
    render: (properties) => (
        <Carousel {...properties}>
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
            <CarouselPlay />
            <CarouselDots />
        </Carousel>
    ),
});

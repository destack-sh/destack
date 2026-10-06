import { defineExample } from "@destack/package/declare";
import { For } from "@destack/view";
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
    { url: "/photos/alfama.jpg", caption: "Alfama at dusk" },
    { url: "/photos/tram.jpg", caption: "Tram 28 on the hill" },
    { url: "/photos/belem.jpg", caption: "Pastries in Belém" },
];

/** The photos of a note one at a time. */
export const carouselTripPhotos = defineExample({
    of: Carousel,
    name: "trip-photos",
    description: "the photos of a note one at a time",
    render: () => (
        <Carousel aria-label="Photos">
            <CarouselContent>
                <For each={PHOTOS}>
                    {(photo) => (
                        <CarouselItem>
                            <img src={photo.url} alt={photo.caption} />
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
    render: () => (
        <Carousel aria-label="Trip photos" loop autoplay={5000}>
            <CarouselContent>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
                <CarouselItem>Faro</CarouselItem>
            </CarouselContent>
            <CarouselPlay />
            <CarouselDots />
        </Carousel>
    ),
});

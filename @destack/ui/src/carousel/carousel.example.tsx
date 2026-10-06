import { defineExample } from "@destack/package/declare";
import { For } from "solid-js";
import {
    Carousel,
    CarouselContent,
    CarouselItem,
    CarouselNext,
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

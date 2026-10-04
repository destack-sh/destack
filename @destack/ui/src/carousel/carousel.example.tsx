import type { JSX } from "@solidjs/web";
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

/** Show the photos of a note one at a time. */
export function CarouselExample(): JSX.Element {
    return (
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
    );
}

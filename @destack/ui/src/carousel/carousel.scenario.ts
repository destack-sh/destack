import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { carouselTripPhotos } from "./carousel.example.tsx";
import { Carousel } from "./carousel.tsx";

/** Move between slides with the arrow keys. */
export const carouselMoveWithArrowKeys = defineScenario({
    of: Carousel,
    interaction: viewInteraction,
    name: "move-with-arrow-keys",
    description: "move to the next and previous slide with the arrow keys, stopping at the ends",
    given: { examples: [carouselTripPhotos] },
    when: [
        { action: "press", key: "ArrowRight", target: { role: "button", name: "Next slide" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight", target: { role: "button", name: "Previous slide" } },
        { action: "press", key: "ArrowLeft" },
    ],
    then: {
        observe: {
            previous: {
                kind: "state",
                target: { role: "button", name: "Previous slide" },
                state: "disabled",
            },
            next: {
                kind: "state",
                target: { role: "button", name: "Next slide" },
                state: "disabled",
            },
        },
        each: [
            { previous: false, next: false },
            { previous: false, next: true },
            { previous: false, next: true },
            { previous: false, next: false },
        ],
    },
});

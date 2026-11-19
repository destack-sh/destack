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
    parameters: {},
    when: [
        {
            step: {
                action: "press",
                key: "ArrowRight",
                target: { role: "button", name: "Next slide" },
            },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Previous slide" },
                        state: "disabled",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Next slide" },
                        state: "disabled",
                    },
                    is: false,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Previous slide" },
                        state: "disabled",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Next slide" },
                        state: "disabled",
                    },
                    is: true,
                },
            ],
        },
        {
            step: {
                action: "press",
                key: "ArrowRight",
                target: { role: "button", name: "Previous slide" },
            },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Previous slide" },
                        state: "disabled",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Next slide" },
                        state: "disabled",
                    },
                    is: true,
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Previous slide" },
                        state: "disabled",
                    },
                    is: false,
                },
                {
                    observe: {
                        kind: "state",
                        target: { role: "button", name: "Next slide" },
                        state: "disabled",
                    },
                    is: false,
                },
            ],
        },
    ],
});

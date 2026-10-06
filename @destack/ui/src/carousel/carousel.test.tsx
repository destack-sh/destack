import { expect, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import {
    Carousel,
    type CarouselApi,
    CarouselContent,
    CarouselItem,
    CarouselNext,
    CarouselPrevious,
} from "./index.ts";
import { draw } from "@destack/view/test";

test("move between a carousel's slides with its buttons, naming each slide by its place", () => {
    const container = draw(() => (
        <Carousel aria-label="Photos">
            <CarouselContent>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
                <CarouselItem>Faro</CarouselItem>
            </CarouselContent>
            <CarouselPrevious />
            <CarouselNext />
        </Carousel>
    ));
    const previous = container.querySelector<HTMLButtonElement>("[data-slot=carousel-previous]");
    const next = container.querySelector<HTMLButtonElement>("[data-slot=carousel-next]");
    const states = () => [previous?.disabled, next?.disabled];
    const atStart = states();
    next?.click();
    flush();
    expect([atStart, states()]).toEqual([
        [true, false],
        [false, false],
    ]);
    expect(
        [...container.querySelectorAll("[role=group]")].map((slide) =>
            slide.getAttribute("aria-label"),
        ),
    ).toEqual(["1 of 3", "2 of 3", "3 of 3"]);
    expect(container.firstElementChild?.getAttribute("aria-roledescription")).toBe("carousel");
});

test("follow a controlled index, reporting each move the API, buttons or keys ask for", () => {
    const [index, setIndex] = createSignal(1);
    const changes: number[] = [];
    let api: CarouselApi | undefined;
    const container = draw(() => (
        <Carousel
            aria-label="Photos"
            index={index()}
            onIndexChange={(next) => changes.push(next)}
            api={(handed) => (api = handed)}
        >
            <CarouselContent>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
                <CarouselItem>Faro</CarouselItem>
            </CarouselContent>
            <CarouselPrevious />
            <CarouselNext />
        </Carousel>
    ));
    const previous = container.querySelector<HTMLButtonElement>("[data-slot=carousel-previous]");
    const startedInMiddle = previous?.disabled;
    api?.scrollNext();
    flush();
    const asked = [...changes];
    setIndex(0);
    flush();

    // the owner's index decides the slide shown, the API only asks
    expect([startedInMiddle, asked, api?.index(), api?.count(), previous?.disabled]).toEqual([
        false,
        [2],
        0,
        3,
        true,
    ]);
});

import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import {
    Carousel,
    type CarouselApi,
    CarouselContent,
    CarouselDots,
    CarouselItem,
    CarouselNext,
    CarouselPlay,
    CarouselPrevious,
} from "./index.ts";
import { draw, wait } from "@destack/view/test";

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

test("wrap a looping carousel from its last slide to its first, and move to a slide by its dot", () => {
    const container = draw(() => (
        <Carousel aria-label="Photos" loop defaultIndex={2}>
            <CarouselContent>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
                <CarouselItem>Faro</CarouselItem>
            </CarouselContent>
            <CarouselNext />
            <CarouselDots />
        </Carousel>
    ));
    const current = (): string | null | undefined =>
        container
            .querySelector("[data-slot=carousel-dot][aria-current]")
            ?.getAttribute("aria-label");
    const next = container.querySelector<HTMLButtonElement>("[data-slot=carousel-next]");
    next?.click();
    flush();
    const wrapped = [current(), next?.disabled];
    container.querySelectorAll<HTMLElement>("[data-slot=carousel-dot]")[1]?.click();
    flush();
    expect([wrapped, current()]).toEqual([["Go to slide 1", false], "Go to slide 2"]);
});

test("advance an autoplaying carousel until its play button stops it, pausing its live region while it rotates", async () => {
    const container = draw(() => (
        <Carousel aria-label="Photos" autoplay={20}>
            <CarouselContent>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
                <CarouselItem>Faro</CarouselItem>
            </CarouselContent>
            <CarouselPlay />
            <CarouselDots />
        </Carousel>
    ));
    const content = container.querySelector("[data-slot=carousel-content]");
    const live = content?.getAttribute("aria-live");
    await wait(50);
    flush();
    const advanced =
        container
            .querySelector("[data-slot=carousel-dot][aria-current]")
            ?.getAttribute("aria-label") !== "Go to slide 1";
    container.querySelector<HTMLElement>("[data-slot=carousel-play]")?.click();
    flush();
    expect([live, advanced, content?.getAttribute("aria-live")]).toEqual(["off", true, "polite"]);
});

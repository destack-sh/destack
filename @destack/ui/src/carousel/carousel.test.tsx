import { expect, test } from "@destack/test";
import { createSignal, flush, For, Show } from "@destack/view";
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
import { render, wait } from "@destack/view/test";

test("move between a carousel's slides with its buttons, naming each slide by its place", () => {
    const { container } = render(() => (
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

test("name each slide by its place as slides come and go among the others", async () => {
    // show a slide before the others, then hide it again
    const [isFirst, setFirst] = createSignal(false);
    const { container } = render(() => (
        <Carousel aria-label="Photos">
            <CarouselContent>
                <Show when={isFirst()}>
                    <CarouselItem>Braga</CarouselItem>
                </Show>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
            </CarouselContent>
        </Carousel>
    ));
    const labels = () =>
        [...container.querySelectorAll("[role=group]")].map((slide) => [
            slide.textContent,
            slide.getAttribute("aria-label"),
        ]);
    setFirst(true);
    flush();
    await Promise.resolve();
    flush();
    const shown = labels();
    setFirst(false);
    flush();

    expect([shown, labels()]).toEqual([
        [
            ["Braga", "1 of 3"],
            ["Lisbon", "2 of 3"],
            ["Porto", "3 of 3"],
        ],
        [
            ["Lisbon", "1 of 2"],
            ["Porto", "2 of 2"],
        ],
    ]);
});

test("name each slide by its place after the slides move without mounting", async () => {
    // move the last slide first, which moves its element and mounts nothing
    const [cities, setCities] = createSignal(["Lisbon", "Porto", "Faro"]);
    const { container } = render(() => (
        <Carousel aria-label="Photos">
            <CarouselContent>
                <For each={cities()}>{(city) => <CarouselItem>{city}</CarouselItem>}</For>
            </CarouselContent>
        </Carousel>
    ));
    const slides = () => [...container.querySelectorAll("[role=group]")];
    const before = new Set(slides());
    setCities(["Faro", "Lisbon", "Porto"]);
    flush();
    await wait(0);
    flush();

    expect({
        labels: slides().map((slide) => [slide.textContent, slide.getAttribute("aria-label")]),
        isMoved: slides().every((slide) => before.has(slide)),
    }).toEqual({
        labels: [
            ["Faro", "1 of 3"],
            ["Lisbon", "2 of 3"],
            ["Porto", "3 of 3"],
        ],
        isMoved: true,
    });
});

test("follow a controlled index, reporting each move the API, buttons or keys ask for", () => {
    const [index, setIndex] = createSignal(1);
    const changes: number[] = [];
    let api: CarouselApi | undefined;
    const { container } = render(() => (
        <Carousel
            aria-label="Photos"
            index={index()}
            onIndexChange={(next) => changes.push(next)}
            setApi={(handed) => (api = handed)}
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
    const { container } = render(() => (
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
    const { container } = render(() => (
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

    // poll until a tick leaves the first slide, however late the timers run
    const current = () =>
        container
            .querySelector("[data-slot=carousel-dot][aria-current]")
            ?.getAttribute("aria-label");
    for (let waited = 0; current() === "Go to slide 1" && waited < 1000; waited += 10) {
        await wait(10);
        flush();
    }
    const hasAdvanced = current() !== "Go to slide 1";
    container.querySelector<HTMLElement>("[data-slot=carousel-play]")?.click();
    flush();
    expect([live, hasAdvanced, content?.getAttribute("aria-live")]).toEqual([
        "off",
        true,
        "polite",
    ]);
});

test("let the keyboard reach a carousel's scrolling content, which announces slides while it rests", () => {
    // a resting carousel of two slides
    const { container } = render(() => (
        <Carousel aria-label="Photos">
            <CarouselContent>
                <CarouselItem>Lisbon</CarouselItem>
                <CarouselItem>Porto</CarouselItem>
            </CarouselContent>
        </Carousel>
    ));
    const content = container.querySelector("[data-slot=carousel-content]");

    expect([content?.getAttribute("tabindex"), content?.getAttribute("aria-live")]).toEqual([
        "0",
        "polite",
    ]);
});

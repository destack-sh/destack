import { present } from "@destack/schema";
import { expect, test, vi } from "@destack/test";
import { ActionRegistry, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import {
    AudioPlayer,
    Player,
    PlayerAudio,
    PlayerControl,
    PlayerTitle,
    seekTo,
    usePlayer,
    VideoPlayer,
} from "./index.ts";

/** Draw a film of a minute that plays and pauses at once. */
function draw() {
    // play and pause at once
    stubPopovers();
    const { container } = render(() => (
        <VideoPlayer source="/film.mp4" title="The pass" length={65} />
    ));
    const film = present(container.querySelector("video"), "the film");
    const isPaused = { value: true };
    Object.defineProperties(film, {
        paused: { get: () => isPaused.value },
        duration: { get: () => 65 },
        readyState: { get: () => HTMLMediaElement.HAVE_ENOUGH_DATA },
    });
    const tell = (name: string) => {
        film.dispatchEvent(new Event(name));
        flush();
    };
    vi.spyOn(film, "play").mockImplementation(async () => {
        isPaused.value = false;
        tell("play");
    });
    vi.spyOn(film, "pause").mockImplementation(() => {
        isPaused.value = true;
        tell("pause");
    });
    tell("loadedmetadata");
    const press = (key: string) => {
        const frame = present(container.querySelector("[data-slot=player-frame]"), "the frame");
        frame.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
        flush();

        return [film.paused, Math.round(film.currentTime), film.muted];
    };
    const time = () => container.querySelector("[data-slot=player-time]")?.textContent;

    return { container, film, press, time, tell };
}

test("play, skip and mute by keys", () => {
    const { press } = draw();

    expect([" ", "l", "ArrowRight", "j", "m", "k"].map(press)).toEqual([
        [false, 0, false],
        [false, 10, false],
        [false, 20, false],
        [false, 10, false],
        [false, 10, true],
        [true, 10, true],
    ]);
});

test("stay within the length", () => {
    const { film, press } = draw();
    film.currentTime = 60;

    expect(["l", "j", "j", "j", "j", "j", "j", "j"].map(press).map(([, at]) => at)).toEqual([
        65, 55, 45, 35, 25, 15, 5, 0,
    ]);
});

test("write and say the time", () => {
    const { container, film, time, tell } = draw();
    const before = time();
    film.currentTime = 64;
    tell("timeupdate");
    const thumb = container.querySelector("[data-slot=player-seek-bar] input");

    expect([before, time(), thumb?.getAttribute("aria-valuetext")]).toEqual([
        "0:00 / 1:05",
        "1:04 / 1:05",
        "1 minute and 4 seconds",
    ]);
});

test("tell of a film that fails, and try it again", () => {
    const { container, film, tell } = draw();
    const load = vi.spyOn(film, "load").mockImplementation(() => {});
    Object.defineProperty(film, "error", { get: () => ({ code: 4 }), configurable: true });
    tell("error");
    const alert = container.querySelector("[role=alert]")?.textContent;
    container.querySelector<HTMLElement>("[role=alert] button")?.click();

    expect([alert, load.mock.calls.length]).toEqual(["This could not be played.Try again", 1]);
});

test("pause as another sound starts", () => {
    stubPopovers();
    const { container } = render(() => (
        <>
            <AudioPlayer source="/one.mp3" />
            <AudioPlayer source="/two.mp3" />
        </>
    ));
    const [one, two] = container.querySelectorAll("audio");
    const paused = [one, two].map((each) =>
        vi.spyOn(present(each, "a sound"), "pause").mockImplementation(() => {}),
    );
    two?.dispatchEvent(new Event("play"));

    expect(paused.map((each) => each.mock.calls.length)).toEqual([1, 0]);
});

test("offer what comes before and after", () => {
    stubPopovers();
    const ran: string[] = [];
    const { container } = render(() => (
        <AudioPlayer
            source="/one.mp3"
            onPrevious={() => ran.push("previous")}
            onNext={() => ran.push("next")}
            onEnded={() => ran.push("ended")}
        />
    ));
    for (const label of ["Previous", "Next"]) {
        container.querySelector<HTMLElement>(`[aria-label=${label}]`)?.click();
    }
    container.querySelector("audio")?.dispatchEvent(new Event("ended"));

    expect(ran).toEqual(["previous", "next", "ended"]);
});

test("go to a time by its action", async () => {
    stubPopovers();
    const actions = new ActionRegistry();
    const control = new PlayerControl();
    const { container } = render(() => <AudioPlayer source="/one.mp3" control={control} />, {
        actions,
    });
    await actions.run(seekTo, container.querySelector("[data-slot=audio-player]"), { time: 80 });

    expect(control.time()).toBe(80);
});

/** Tell how much of a sound is left. */
function Left() {
    const player = usePlayer();

    return <b>{String((player.length() ?? 0) - player.time())}</b>;
}

test("play by parts of its own", () => {
    stubPopovers();
    const { container } = render(() => (
        <Player source="/one.mp3" title="First Tram" length={12}>
            <PlayerAudio />
            <PlayerTitle />
            <Left />
        </Player>
    ));

    expect([container.textContent, container.querySelector("audio")?.getAttribute("src")]).toEqual([
        "First Tram12",
        "/one.mp3",
    ]);
});

test("take the focus once, as it starts", async () => {
    const { container, film, tell } = draw();
    container.querySelector<HTMLElement>("[data-slot=player-start-button]")?.click();
    await Promise.resolve();
    flush();
    await Promise.resolve();
    const isTaken = document.activeElement === container.querySelector("[data-slot=player-frame]");
    present(container.querySelector<HTMLElement>("[data-slot=player-frame]"), "the frame").blur();
    film.currentTime = 4;
    tell("timeupdate");
    await Promise.resolve();

    expect([isTaken, document.activeElement === document.body]).toEqual([true, true]);
});

import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, createSignal, flush, NotReadyError } from "solid-js";
import { createAudio, makeAudio, makeAudioPlayer } from "./audio.ts";

/** The media element methods and properties the stand-in replaces, restored after each test. */
const REPLACED = ["play", "pause", "paused", "duration"] as const;

/** The originals of the replaced members. */
const originals = new Map<string, PropertyDescriptor | undefined>();

/** Whether each player is paused, keyed by player. */
const paused = new WeakMap<HTMLMediaElement, boolean>();

beforeEach(() => {
    // stand in for playback: playing loads the sound, and both dispatch their events
    for (const name of REPLACED) {
        originals.set(name, Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, name));
    }
    Object.defineProperty(HTMLMediaElement.prototype, "paused", {
        configurable: true,
        get: function (this: HTMLMediaElement) {
            return paused.get(this) ?? true;
        },
    });
    Object.defineProperty(HTMLMediaElement.prototype, "duration", {
        configurable: true,
        value: 300,
    });
    Object.defineProperty(HTMLMediaElement.prototype, "play", {
        configurable: true,
        value: async function (this: HTMLMediaElement) {
            this.dispatchEvent(new Event("loadeddata"));
            paused.set(this, false);
            this.dispatchEvent(new Event("playing"));
        },
    });
    Object.defineProperty(HTMLMediaElement.prototype, "pause", {
        configurable: true,
        value: function (this: HTMLMediaElement) {
            paused.set(this, true);
            this.dispatchEvent(new Event("pause"));
        },
    });
});

afterEach(() => {
    for (const [name, descriptor] of originals) {
        if (descriptor === undefined) {
            Reflect.deleteProperty(HTMLMediaElement.prototype, name);
        } else {
            Object.defineProperty(HTMLMediaElement.prototype, name, descriptor);
        }
    }
});

test("make a player with handlers, removed and paused by its cleanup", async () => {
    const seen: string[] = [];
    const [player, cleanup] = makeAudio("sound.mp3", { playing: (event) => seen.push(event.type) });
    await player?.play();
    cleanup();
    await player?.play();

    expect([player?.src.endsWith("sound.mp3"), seen]).toEqual([true, ["playing"]]);
});

test("control a player: play, pause, seek and set the volume", async () => {
    const [controls, cleanup] = makeAudioPlayer("sound.mp3");
    await controls.play();
    const isPlaying = controls.player?.paused === false;
    controls.pause();
    controls.seek(12);
    controls.setVolume(0.3);
    const state = [controls.player?.paused, controls.player?.currentTime, controls.player?.volume];
    cleanup();

    expect([isPlaying, state]).toEqual([true, [true, 12, 0.3]]);
});

test("follow whether a sound plays, its volume and its length, pausing on cleanup", async () => {
    // read before playing
    const { audio, dispose } = createRoot((disposeRoot) => ({
        audio: createAudio("sound.mp3"),
        dispose: disposeRoot,
    }));
    const before = (() => {
        try {
            return audio.duration();
        } catch (error) {
            return error instanceof NotReadyError ? "pending" : error;
        }
    })();

    // play, set the volume, then pause
    audio.setPlaying(true);
    await Promise.resolve();
    flush();
    const during = [audio.playing(), audio.duration()];
    audio.setVolume(0.4);
    audio.player?.dispatchEvent(new Event("volumechange"));
    flush();
    audio.setPlaying(false);
    flush();
    const after = [audio.playing(), audio.volume()];
    dispose();

    expect({ before, during, after, isPaused: audio.player?.paused }).toEqual({
        before: "pending",
        during: [true, 300],
        after: [false, 0.4],
        isPaused: true,
    });
});

test("follow a reactive source, seeking to the start of each", () => {
    const [source, setSource] = createSignal("first.mp3", { ownedWrite: true });
    const observed = createRoot((disposeRoot) => {
        const audio = createAudio(source);
        flush();
        const first = audio.player?.src.endsWith("first.mp3");
        if (audio.player !== undefined) {
            audio.player.currentTime = 30;
        }
        setSource("second.mp3");
        flush();
        const second = [audio.player?.src.endsWith("second.mp3"), audio.player?.currentTime];
        disposeRoot();

        return [first, second];
    });

    expect(observed).toEqual([true, [true, 0]]);
});

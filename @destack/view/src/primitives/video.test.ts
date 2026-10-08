import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, createSignal, flush, NotReadyError } from "solid-js";
import {
    createVideo,
    createVideoFrameCallback,
    createVideoPlayer,
    makeVideo,
    makeVideoPlayer,
    setVideoSrc,
} from "./video.ts";

/** The media element members the stand-in replaces, restored after each test. */
const REPLACED = ["play", "pause", "paused", "duration"] as const;

/** The originals of the replaced members. */
const originals = new Map<string, PropertyDescriptor | undefined>();

/** Whether each player is paused. */
const paused = new WeakMap<HTMLMediaElement, boolean>();

beforeEach(() => {
    // stand in for playback: playing loads the metadata, and both dispatch their events
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
        value: 90,
    });
    Object.defineProperty(HTMLMediaElement.prototype, "play", {
        configurable: true,
        value: async function (this: HTMLMediaElement) {
            this.dispatchEvent(new Event("loadedmetadata"));
            paused.set(this, false);
            this.dispatchEvent(new Event("play"));
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

test("make a configured player with handlers, removed and paused by its cleanup", async () => {
    const seen: string[] = [];
    const [player, cleanup] = makeVideo(
        "clip.mp4",
        { playing: (event) => seen.push(event.type) },
        { muted: true, loop: true, preload: "metadata" },
    );
    await player?.play();
    cleanup();
    await player?.play();

    expect([player?.muted, player?.loop, player?.preload, seen]).toEqual([
        true,
        true,
        "metadata",
        ["playing"],
    ]);
});

test("give a player an address or a provider, clearing the other", () => {
    const player = document.createElement("video");
    setVideoSrc(player, "clip.mp4");
    const address = [player.src.endsWith("clip.mp4"), player.srcObject];
    setVideoSrc(player, undefined);

    expect([address, player.getAttribute("src"), player.srcObject]).toEqual([
        [true, null],
        "",
        null,
    ]);
});

test("control a player: play, pause, seek, volume, muting, rate and looping", async () => {
    const [controls, cleanup] = makeVideoPlayer("clip.mp4");
    await controls.play();
    controls.pause();
    controls.seek(12);
    controls.setVolume(0.3);
    controls.setMuted(true);
    controls.setPlaybackRate(1.5);
    controls.setLoop(true);
    const { player } = controls;
    const state = [
        player?.paused,
        player?.currentTime,
        player?.volume,
        player?.muted,
        player?.playbackRate,
        player?.loop,
    ];
    cleanup();

    expect(state).toEqual([true, 12, 0.3, true, 1.5, true]);
});

test("follow a video's playback, end, seeking, error and length", async () => {
    // read before playing
    const { video, dispose } = createRoot((disposeRoot) => ({
        video: createVideo("clip.mp4"),
        dispose: disposeRoot,
    }));
    let pending: unknown;
    try {
        pending = video.duration();
    } catch (error) {
        pending = error instanceof NotReadyError ? "pending" : error;
    }

    // play, seek, end, then start loading another
    video.setPlaying(true);
    await Promise.resolve();
    flush();
    const playing = [video.playing(), video.duration(), video.ended()];
    video.player?.dispatchEvent(new Event("seeking"));
    flush();
    const isSeeking = video.seeking();
    video.player?.dispatchEvent(new Event("seeked"));
    video.player?.dispatchEvent(new Event("ended"));
    flush();
    const ended = [video.playing(), video.ended(), video.seeking()];
    dispose();

    expect({ pending, playing, isSeeking, ended, isPaused: video.player?.paused }).toEqual({
        pending: "pending",
        playing: [true, 90, false],
        isSeeking: true,
        ended: [false, true, false],
        isPaused: true,
    });
});

test("follow a video's volume, muting, rate, looping and readiness, from its first configuration", () => {
    const observed = createRoot((disposeRoot) => {
        const video = createVideoPlayer("clip.mp4", { volume: 0.5, playbackRate: 2 });
        const initial = [video.volume(), video.playbackRate(), video.loop()];
        video.setVolume(0.2);
        video.setMuted(true);
        video.player?.dispatchEvent(new Event("volumechange"));
        video.setPlaybackRate(0.5);
        video.player?.dispatchEvent(new Event("ratechange"));
        video.setLoop(true);
        flush();
        const changed = [
            video.volume(),
            video.muted(),
            video.playbackRate(),
            video.loop(),
            video.readyState(),
        ];
        disposeRoot();

        return [initial, changed];
    });

    expect(observed).toEqual([
        [0.5, 2, false],
        [0.2, true, 0.5, true, 0],
    ]);
});

test("follow a reactive source, seeking to the start of each", () => {
    const [source, setSource] = createSignal("first.mp4", { ownedWrite: true });
    const observed = createRoot((disposeRoot) => {
        const video = createVideo(source);
        flush();
        if (video.player !== undefined) {
            video.player.currentTime = 30;
        }
        setSource("second.mp4");
        flush();
        const read = [video.player?.src.endsWith("second.mp4"), video.player?.currentTime];
        disposeRoot();

        return read;
    });

    expect(observed).toEqual([true, 0]);
});

test("call back once per frame of the video an accessor gives while started", () => {
    // stand in for frame callbacks on two videos
    const requests = new Map<HTMLVideoElement, VideoFrameRequestCallback[]>();
    const videos = [document.createElement("video"), document.createElement("video")];
    for (const video of videos) {
        requests.set(video, []);
        Object.assign(video, {
            requestVideoFrameCallback: (callback: VideoFrameRequestCallback) =>
                requests.get(video)?.push(callback),
            cancelVideoFrameCallback: () => requests.set(video, []),
        });
    }
    const frame = (video: HTMLVideoElement | undefined): void => {
        const pending = video === undefined ? [] : (requests.get(video) ?? []);
        if (video !== undefined) {
            requests.set(video, []);
        }
        for (const callback of pending) {
            callback(0, {
                expectedDisplayTime: 0,
                height: 0,
                mediaTime: 0,
                presentationTime: 0,
                presentedFrames: 1,
                width: 0,
            });
        }
    };

    // run on the first video, then move to the second
    let frames = 0;
    const [current, setCurrent] = createSignal<HTMLVideoElement | undefined>(videos[0], {
        ownedWrite: true,
    });
    const observed = createRoot((disposeRoot) => {
        const [running, start, stop] = createVideoFrameCallback(current, () => frames++);
        start();
        frame(videos[0]);
        setCurrent(videos[1]);
        flush();
        frame(videos[0]);
        frame(videos[1]);
        stop();
        frame(videos[1]);
        flush();
        const isRunning = running();
        disposeRoot();

        return isRunning;
    });

    expect([frames, observed]).toEqual([2, false]);
});

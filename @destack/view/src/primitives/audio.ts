import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal, NotReadyError, onCleanup } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { access, TRANSPARENT } from "./utils.ts";

/** A sound to play: a file's address, a media provider such as a stream, or nothing. */
export type AudioSource = string | undefined | MediaProvider;

/** Handlers of a player's media events. */
export type AudioEventHandlers = {
    [Name in keyof HTMLMediaElementEventMap]?: (event: HTMLMediaElementEventMap[Name]) => void;
};

/** A player's controls. */
export type AudioControls = {
    /** Play, rejecting where the browser refuses. */
    readonly play: () => Promise<void>;
    /** Pause. */
    readonly pause: () => void;
    /** Move to a time in seconds. */
    readonly seek: (time: number) => void;
    /** Set the volume, from zero to one. */
    readonly setVolume: (volume: number) => void;
    /** The player, absent on the server. */
    readonly player: HTMLAudioElement | undefined;
};

/** A reactive player: its state, the controls that change it, and the player itself. */
export type AudioReturn = {
    /** The player, absent on the server. */
    readonly player: HTMLAudioElement | undefined;
    /** Whether it plays. */
    readonly playing: Accessor<boolean>;
    /** Play or pause, reporting a refused play. */
    readonly setPlaying: (isPlaying: boolean) => void;
    /** The volume, from zero to one. */
    readonly volume: Accessor<number>;
    /** Set the volume, from zero to one. */
    readonly setVolume: (volume: number) => void;
    /** The time played, in seconds. */
    readonly currentTime: Accessor<number>;
    /** The length in seconds, pending until the sound loads and again once its source changes. */
    readonly duration: Accessor<number>;
    /** Move to a time in seconds. */
    readonly seek: (time: number) => void;
};

/** Make a player of a sound, or take an existing one, with event handlers removed by the returned function, which also pauses it. */
export function makeAudio(
    source: AudioSource | HTMLAudioElement,
    handlers: AudioEventHandlers = {},
): [player: HTMLAudioElement | undefined, cleanup: () => void] {
    // play nothing on the server
    if (isServer) {
        return [undefined, () => {}];
    }

    // listen to each handler's event until cleaned up
    const player = playerOf(source);
    const stops = Object.entries(handlers).map(([name, handler]) => {
        const listener: EventListenerObject = { handleEvent: handler };
        player.addEventListener(name, listener);

        return () => player.removeEventListener(name, listener);
    });

    return [
        player,
        () => {
            player.pause();
            for (const stop of stops) {
                stop();
            }
        },
    ];
}

/** Make a player of a sound with simple controls, cleaned up by the returned function. */
export function makeAudioPlayer(
    source: AudioSource | HTMLAudioElement,
    handlers: AudioEventHandlers = {},
): [controls: AudioControls, cleanup: () => void] {
    const [player, cleanup] = makeAudio(source, handlers);

    // control nothing on the server
    if (player === undefined) {
        return [
            { play: async () => {}, pause: () => {}, seek: () => {}, setVolume: () => {}, player },
            cleanup,
        ];
    }

    return [
        {
            player,
            play: async () => player.play(),
            pause: () => player.pause(),
            seek: (time) => {
                player.currentTime = time;
            },
            setVolume: (volume) => {
                player.volume = volume;
            },
        },
        cleanup,
    ];
}

/** Play a sound reactively: whether it plays, its volume, time and length, following a reactive source, pausing on cleanup. */
export function createAudio(source: AudioSource | Accessor<AudioSource>): AudioReturn {
    // play nothing on the server
    if (isServer) {
        return {
            player: undefined,
            playing: () => false,
            volume: () => 1,
            currentTime: () => 0,
            duration: () => {
                throw new NotReadyError("audio duration is not available on the server");
            },
            setPlaying: () => {},
            setVolume: () => {},
            seek: () => {},
        };
    }
    const player = playerOf(access(source));
    const [controls, cleanup] = makeAudioPlayer(player);
    onCleanup(cleanup);

    // follow the time, whether it plays, and the volume through the player's events
    const [currentTime, setCurrentTime] = createSignal(0, { ownedWrite: true });
    const [playing, setPlaying] = createSignal(!player.paused, { ownedWrite: true });
    const [volume, setVolume] = createSignal(player.volume, { ownedWrite: true });
    makeEventListener(player, "timeupdate", () => setCurrentTime(player.currentTime));
    makeEventListener(player, "playing", () => setPlaying(true));
    makeEventListener(player, "pause", () => setPlaying(false));
    makeEventListener(player, "ended", () => setPlaying(false));
    makeEventListener(player, "volumechange", () => setVolume(player.volume));

    // follow the length, pending until it loads and again once a new source starts loading
    const [length, setLength] = createSignal<number | undefined>(undefined, { ownedWrite: true });
    makeEventListener(player, "loadeddata", () => setLength(player.duration));
    makeEventListener(player, "loadstart", () => setLength(undefined));
    const duration = (): number => {
        const current = length();
        if (current === undefined) {
            throw new NotReadyError("audio duration is not available yet");
        }

        return current;
    };

    // follow a reactive source from the start
    if (typeof source === "function") {
        createEffect(
            source,
            (next) => {
                sourceInto(player, next);
                controls.seek(0);
            },
            TRANSPARENT,
        );
    }

    return {
        player,
        playing,
        setPlaying: (isPlaying) => {
            if (isPlaying) {
                controls.play().catch(reportError);
            } else {
                controls.pause();
            }
        },
        volume,
        setVolume: controls.setVolume,
        currentTime,
        duration,
        seek: controls.seek,
    };
}

/** Take an existing player, or make one of a sound. */
function playerOf(source: AudioSource | HTMLAudioElement): HTMLAudioElement {
    if (source instanceof HTMLAudioElement) {
        return source;
    }
    const player = new Audio();
    sourceInto(player, source);

    return player;
}

/** Give a player a sound: an address as its source, or a provider as its source object. */
function sourceInto(player: HTMLAudioElement, source: AudioSource): void {
    if (typeof source === "string") {
        player.src = source;
    } else {
        player.srcObject = source ?? null;
    }
}

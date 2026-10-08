import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createSignal,
    NotReadyError,
    onCleanup,
    untrack,
} from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { access, TRANSPARENT } from "./utils.ts";

/** A video to play: a file's address, a media provider such as a stream, or nothing. */
export type VideoSource = string | undefined | MediaProvider;

/** Handlers of a video player's events. */
export type VideoEventHandlers = {
    [Name in keyof HTMLVideoElementEventMap]?: (event: HTMLVideoElementEventMap[Name]) => void;
};

/** The player's first configuration. */
export type VideoOptions = {
    /** Whether it plays once it can. */
    readonly autoPlay?: boolean;
    /** Whether it starts over at the end. */
    readonly loop?: boolean;
    /** Whether it is muted. */
    readonly muted?: boolean;
    /** How much it loads before playing. */
    readonly preload?: "" | "none" | "metadata" | "auto";
};

/** The player's first configuration, with its volume and playback rate. */
export type VideoControlsOptions = VideoOptions & {
    /** The volume, from zero to one. */
    readonly volume?: number;
    /** The playback rate. */
    readonly playbackRate?: number;
};

/** A video player's controls. */
export type VideoControls = {
    /** Play, rejecting where the browser refuses. */
    readonly play: () => Promise<void>;
    /** Pause. */
    readonly pause: () => void;
    /** Move to a time in seconds. */
    readonly seek: (time: number) => void;
    /** Set the volume, from zero to one. */
    readonly setVolume: (volume: number) => void;
    /** Mute or unmute. */
    readonly setMuted: (muted: boolean) => void;
    /** Set the playback rate. */
    readonly setPlaybackRate: (rate: number) => void;
    /** Start over at the end, or not. */
    readonly setLoop: (loop: boolean) => void;
    /** The player, absent on the server. */
    readonly player: HTMLVideoElement | undefined;
};

/** A reactive video's playback state. */
export type VideoReturn = {
    /** The player, absent on the server. */
    readonly player: HTMLVideoElement | undefined;
    /** Whether it plays. */
    readonly playing: Accessor<boolean>;
    /** Play or pause, reporting a refused play. */
    readonly setPlaying: (isPlaying: boolean) => void;
    /** The time played, in seconds. */
    readonly currentTime: Accessor<number>;
    /** Move to a time in seconds. */
    readonly seek: (time: number) => void;
    /** Whether it reached its end. */
    readonly ended: Accessor<boolean>;
    /** Whether it seeks. */
    readonly seeking: Accessor<boolean>;
    /** The media error it met, null while none. */
    readonly error: Accessor<MediaError | null>;
    /** The length in seconds, pending until its metadata loads and again once its source changes. */
    readonly duration: Accessor<number>;
};

/** A reactive video's playback state with every control. */
export type VideoControlsReturn = VideoReturn & {
    /** The volume, from zero to one. */
    readonly volume: Accessor<number>;
    /** Set the volume, from zero to one. */
    readonly setVolume: (volume: number) => void;
    /** Whether it is muted. */
    readonly muted: Accessor<boolean>;
    /** Mute or unmute. */
    readonly setMuted: (muted: boolean) => void;
    /** The playback rate. */
    readonly playbackRate: Accessor<number>;
    /** Set the playback rate. */
    readonly setPlaybackRate: (rate: number) => void;
    /** Whether it starts over at the end. */
    readonly loop: Accessor<boolean>;
    /** Start over at the end, or not. */
    readonly setLoop: (loop: boolean) => void;
    /** The buffered ranges, undefined before the first progress. */
    readonly buffered: Accessor<TimeRanges | undefined>;
    /** How much it has loaded, from nothing at 0 to enough at 4. */
    readonly readyState: Accessor<number>;
    /** The video's intrinsic width. */
    readonly videoWidth: Accessor<number>;
    /** The video's intrinsic height. */
    readonly videoHeight: Accessor<number>;
};

/** Give a player a video: an address as its source, or a provider as its source object. */
export function setVideoSrc(player: HTMLVideoElement, source: VideoSource): void {
    if (typeof source === "string") {
        player.srcObject = null;
        player.src = source;
    } else {
        player.src = "";
        player.srcObject = source ?? null;
    }
}

/** Make a player of a video, or take an existing one, configured, with handlers removed by the returned function, which also pauses it. */
export function makeVideo(
    source: VideoSource | HTMLVideoElement,
    handlers: VideoEventHandlers = {},
    options?: VideoOptions,
): [player: HTMLVideoElement | undefined, cleanup: () => void] {
    // play nothing on the server
    if (isServer) {
        return [undefined, () => {}];
    }

    // configure the player and listen to each handler's event until cleaned up
    const player = playerOf(source);
    configure(player, options);
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

/** Make a player of a video with every control, cleaned up by the returned function. */
export function makeVideoPlayer(
    source: VideoSource | HTMLVideoElement,
    handlers: VideoEventHandlers = {},
    options?: VideoOptions,
): [controls: VideoControls, cleanup: () => void] {
    const [player, cleanup] = makeVideo(source, handlers, options);

    // control nothing on the server
    if (player === undefined) {
        return [
            {
                play: async () => {},
                pause: none,
                seek: none,
                setVolume: none,
                setMuted: none,
                setPlaybackRate: none,
                setLoop: none,
                player,
            },
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
            setMuted: (muted) => {
                player.muted = muted;
            },
            setPlaybackRate: (rate) => {
                player.playbackRate = rate;
            },
            setLoop: (loop) => {
                player.loop = loop;
            },
        },
        cleanup,
    ];
}

/** Play a video reactively: whether it plays, its time, end, seeking, error and length, following a reactive source, pausing on cleanup. */
export function createVideo(
    source: VideoSource | Accessor<VideoSource>,
    options?: VideoOptions,
): VideoReturn {
    // play nothing on the server
    if (isServer) {
        return serverVideo();
    }
    const player = playerOf(access(source));
    configure(player, options);
    onCleanup(() => player.pause());
    const seek = (time: number): void => {
        player.currentTime = time;
    };

    // follow the playback through the player's events
    const [playing, setPlaying] = createSignal(!player.paused, { ownedWrite: true });
    const [currentTime, setCurrentTime] = createSignal(0, { ownedWrite: true });
    const [ended, setEnded] = createSignal(false, { ownedWrite: true });
    const [seeking, setSeeking] = createSignal(false, { ownedWrite: true });
    const [error, setError] = createSignal<MediaError | null>(null, { ownedWrite: true });
    const [length, setLength] = createSignal<number | undefined>(undefined, { ownedWrite: true });
    const listeners: [keyof HTMLVideoElementEventMap, () => void][] = [
        ["playing", () => setPlaying(true)],
        ["pause", () => setPlaying(false)],
        [
            "ended",
            () => {
                setPlaying(false);
                setEnded(true);
            },
        ],
        ["play", () => setEnded(false)],
        ["timeupdate", () => setCurrentTime(player.currentTime)],
        ["seeking", () => setSeeking(true)],
        ["seeked", () => setSeeking(false)],
        ["error", () => setError(player.error)],
        [
            "loadstart",
            () => {
                setLength(undefined);
                setError(null);
            },
        ],
        ["loadedmetadata", () => setLength(player.duration)],
    ];
    for (const [name, listener] of listeners) {
        makeEventListener(player, name, listener);
    }

    // read the length, pending until the metadata loads
    const duration = (): number => {
        const current = length();
        if (current === undefined) {
            throw new NotReadyError("video duration is not available yet");
        }

        return current;
    };

    // follow a reactive source, seeking to the start of each
    if (typeof source === "function") {
        createEffect(
            source,
            (next) => {
                setVideoSrc(player, next);
                seek(0);
            },
            TRANSPARENT,
        );
    }

    return {
        player,
        playing,
        setPlaying: (isPlaying) => {
            if (isPlaying) {
                player.play().catch(reportError);
            } else {
                player.pause();
            }
        },
        currentTime,
        seek,
        ended,
        seeking,
        error,
        duration,
    };
}

/** Play a video reactively with every control: also its volume, muting, rate, looping, buffering, readiness and size. */
export function createVideoPlayer(
    source: VideoSource | Accessor<VideoSource>,
    options?: VideoControlsOptions,
): VideoControlsReturn {
    // play the base video
    const base = createVideo(source, options);
    const { player } = base;

    // read fixed values on the server
    if (player === undefined) {
        return {
            ...base,
            volume: () => 1,
            setVolume: none,
            muted: () => false,
            setMuted: none,
            playbackRate: () => 1,
            setPlaybackRate: none,
            loop: () => false,
            setLoop: none,
            buffered: () => undefined,
            readyState: () => 0,
            videoWidth: () => 0,
            videoHeight: () => 0,
        };
    }

    // apply the first volume and rate, then follow each control through the player's events
    if (options?.volume !== undefined) {
        player.volume = options.volume;
    }
    if (options?.playbackRate !== undefined) {
        player.playbackRate = options.playbackRate;
    }
    const [volume, setVolume] = createSignal(player.volume, { ownedWrite: true });
    const [muted, setMuted] = createSignal(player.muted, { ownedWrite: true });
    const [playbackRate, setPlaybackRate] = createSignal(player.playbackRate, { ownedWrite: true });
    const [loop, setLoop] = createSignal(player.loop, { ownedWrite: true });
    const [buffered, setBuffered] = createSignal<TimeRanges | undefined>(undefined, {
        ownedWrite: true,
    });
    const [readyState, setReadyState] = createSignal(player.readyState, { ownedWrite: true });
    const [videoWidth, setVideoWidth] = createSignal(player.videoWidth, { ownedWrite: true });
    const [videoHeight, setVideoHeight] = createSignal(player.videoHeight, { ownedWrite: true });
    const syncReadyState = (): void => {
        setReadyState(player.readyState);
    };
    const syncSize = (): void => {
        setVideoWidth(player.videoWidth);
        setVideoHeight(player.videoHeight);
    };

    // listen to the control events, which looping has none of
    const listeners: [keyof HTMLVideoElementEventMap, () => void][] = [
        [
            "volumechange",
            () => {
                setVolume(player.volume);
                setMuted(player.muted);
            },
        ],
        ["ratechange", () => setPlaybackRate(player.playbackRate)],
        ["progress", () => setBuffered(player.buffered)],
        [
            "loadedmetadata",
            () => {
                syncReadyState();
                syncSize();
            },
        ],
        ["loadeddata", syncReadyState],
        ["canplay", syncReadyState],
        ["canplaythrough", syncReadyState],
        ["emptied", syncReadyState],
        ["resize", syncSize],
    ];
    for (const [name, listener] of listeners) {
        makeEventListener(player, name, listener);
    }

    return {
        ...base,
        volume,
        setVolume: (next) => {
            player.volume = next;
        },
        muted,
        setMuted: (next) => {
            player.muted = next;
        },
        playbackRate,
        setPlaybackRate: (next) => {
            player.playbackRate = next;
        },
        loop,
        setLoop: (next) => {
            player.loop = next;
            setLoop(next);
        },
        buffered,
        readyState,
        videoWidth,
        videoHeight,
    };
}

/** Call back once per presented frame of a video while started, with the frame's metadata. */
export function makeVideoFrameCallback(
    video: HTMLVideoElement,
    callback: VideoFrameRequestCallback,
): [running: () => boolean, start: () => void, stop: () => void] {
    // run nothing on the server
    if (isServer) {
        return [() => false, () => {}, () => {}];
    }

    // request the next frame from each frame while running
    let handle = 0;
    let isRunning = false;
    const loop: VideoFrameRequestCallback = (now, metadata) => {
        handle = video.requestVideoFrameCallback(loop);
        callback(now, metadata);
    };

    return [
        () => isRunning,
        () => {
            if (!isRunning) {
                isRunning = true;
                handle = video.requestVideoFrameCallback(loop);
            }
        },
        () => {
            if (isRunning) {
                isRunning = false;
                video.cancelVideoFrameCallback(handle);
            }
        },
    ];
}

/** Call back once per presented frame of the video an accessor gives while started, moving to each new video, stopping on cleanup. */
export function createVideoFrameCallback(
    video: Accessor<HTMLVideoElement | undefined>,
    callback: VideoFrameRequestCallback,
): [running: Accessor<boolean>, start: () => void, stop: () => void] {
    // run nothing on the server
    if (isServer) {
        return [() => false, () => {}, () => {}];
    }

    // attach to each video, carrying over whether it runs
    const [running, setRunning] = createSignal(false, { ownedWrite: true });
    let isActive = false;
    let current: [start: () => void, stop: () => void] = [() => {}, () => {}];
    const attach = (next: HTMLVideoElement | undefined): void => {
        current[1]();
        current = [() => {}, () => {}];
        if (next !== undefined) {
            const [, start, stop] = makeVideoFrameCallback(next, callback);
            current = [start, stop];
            if (isActive) {
                start();
            }
        }
    };
    let attached = untrack(video);
    attach(attached);
    createEffect(
        video,
        (next) => {
            if (next !== attached) {
                attached = next;
                attach(next);
            }
        },
        TRANSPARENT,
    );

    // start and stop, stopping on cleanup
    const stop = (): void => {
        isActive = false;
        setRunning(false);
        current[1]();
    };
    onCleanup(stop);

    return [
        running,
        () => {
            isActive = true;
            setRunning(true);
            current[0]();
        },
        stop,
    ];
}

/** Take an existing player, or make one of a video. */
function playerOf(source: VideoSource | HTMLVideoElement): HTMLVideoElement {
    if (source instanceof HTMLVideoElement) {
        return source;
    }
    const player = document.createElement("video");
    setVideoSrc(player, source);

    return player;
}

/** Apply the first configuration to a player. */
function configure(player: HTMLVideoElement, options: VideoOptions | undefined): void {
    // set each option given, leaving the others to the player
    if (options?.autoPlay !== undefined) {
        player.autoplay = options.autoPlay;
    }
    if (options?.loop !== undefined) {
        player.loop = options.loop;
    }
    if (options?.muted !== undefined) {
        player.muted = options.muted;
    }
    if (options?.preload !== undefined) {
        player.preload = options.preload;
    }
}

/** Read a video's fixed state on the server. */
function serverVideo(): VideoReturn {
    return {
        player: undefined,
        playing: () => false,
        setPlaying: () => {},
        currentTime: () => 0,
        seek: () => {},
        ended: () => false,
        seeking: () => false,
        error: () => null,
        duration: () => {
            throw new NotReadyError("video duration is not available on the server");
        },
    };
}

/** Do nothing, as a control does on the server. */
function none(): void {}

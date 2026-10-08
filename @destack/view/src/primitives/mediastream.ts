import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    onCleanup,
    untrack,
} from "solid-js";
import { access, type FalsyValue, type MaybeAccessor, TRANSPARENT } from "./utils.ts";

/** Where a stream comes from: a device, constraints, an accessor of either, or nothing. */
export type StreamSourceDescription =
    | MediaDeviceInfo
    | MediaStreamConstraints
    | Accessor<MediaDeviceInfo | MediaStreamConstraints | FalsyValue>
    | FalsyValue;

/** How to stop and mute a stream. */
export type StreamControls = {
    /** Stop the stream. */
    readonly stop: () => void;
    /** Mute the stream, or unmute it with false. */
    readonly mute: (muted?: boolean) => void;
};

/** A stream, undefined while it loads or once stopped, and its controls. */
export type StreamReturn = [stream: Accessor<MediaStream | undefined>, controls: StreamControls];

/** The analyser's frequency bins, twice the bins it reads. */
const FFT_SIZE = 128;

/** The quietest level the analyser reads, in decibels. */
const MIN_DECIBELS = -60;

/** The loudest level the analyser reads, in decibels. */
const MAX_DECIBELS = -10;

/** How much of the last reading each reading keeps, from zero to one. */
const SMOOTHING = 0.8;

/** The scale an amplitude reaches at its loudest. */
const MAX_AMPLITUDE = 100;

/** The gain from the frequencies' root mean square to the amplitude scale, which a normal voice fills. */
const AMPLITUDE_GAIN = 4;

/** Get a stream from a device or constraints, again as the source changes, stopping it on cleanup and reporting a refusal. */
export function createStream(streamSource: StreamSourceDescription): StreamReturn {
    // open nothing on the server
    if (isServer) {
        return [() => undefined, { stop: () => {}, mute: () => {} }];
    }
    const [stream, setStream] = createSignal<MediaStream | undefined>(undefined, {
        ownedWrite: true,
    });
    const constraints = createMemo(() => constraintsOf(access(streamSource)), TRANSPARENT);
    followStream(
        constraints,
        async (current) => navigator.mediaDevices.getUserMedia(current),
        stream,
        setStream,
    );

    return [stream, controlsOf(stream, setStream)];
}

/** Capture the screen, a window or a tab, again as the options change, stopping it on cleanup and reporting a refusal. */
export function createScreen(
    screenSource: MaybeAccessor<DisplayMediaStreamOptions | undefined>,
): StreamReturn {
    // open nothing on the server
    if (isServer) {
        return [() => undefined, { stop: () => {}, mute: () => {} }];
    }
    const [stream, setStream] = createSignal<MediaStream | undefined>(undefined, {
        ownedWrite: true,
    });
    followStream(
        () => access(screenSource),
        async (current) => navigator.mediaDevices.getDisplayMedia(current),
        stream,
        setStream,
    );

    return [stream, controlsOf(stream, setStream)];
}

/** Follow a stream's loudness, from zero to a hundred, every frame until stopped or cleaned up. */
export function createAmplitudeFromStream(
    stream: MaybeAccessor<MediaStream | undefined>,
): [amplitude: Accessor<number>, stop: () => void] {
    // analyse nothing on the server
    if (isServer) {
        return [() => 0, () => {}];
    }

    // analyse the stream's frequencies
    const [amplitude, setAmplitude] = createSignal(0, { ownedWrite: true });
    const context = new AudioContext();
    const analyser = context.createAnalyser();
    analyser.fftSize = FFT_SIZE;
    analyser.minDecibels = MIN_DECIBELS;
    analyser.maxDecibels = MAX_DECIBELS;
    analyser.smoothingTimeConstant = SMOOTHING;

    // connect each stream given, resuming the context, reporting a refusal
    let source: MediaStreamAudioSourceNode | undefined;
    createEffect(
        () => access(stream),
        (current) => {
            if (current !== undefined) {
                context.resume().catch(reportError);
                source?.disconnect();
                source = context.createMediaStreamSource(current);
                source.connect(analyser);
            }
        },
        TRANSPARENT,
    );

    // read the root mean square of the frequencies every frame, scaled and capped
    const bins = new Uint8Array(analyser.frequencyBinCount);
    let frame = 0;
    const loop = (): void => {
        // read the frequencies each frame
        frame = requestAnimationFrame(loop);
        analyser.getByteFrequencyData(bins);
        const rootMeanSquare = Math.sqrt(
            bins.reduce((sum, value) => sum + value * value, 0) / bins.length,
        );
        setAmplitude(Math.min(Math.floor(rootMeanSquare) * AMPLITUDE_GAIN, MAX_AMPLITUDE));
    };
    loop();

    // stop reading and close the context
    const stop = (): void => {
        cancelAnimationFrame(frame);
        source?.disconnect();
        if (context.state !== "closed") {
            context.close().catch(reportError);
        }
    };
    onCleanup(stop);

    return [amplitude, stop];
}

/** Get a stream from a device and follow its loudness, stopping both together. */
export function createAmplitudeStream(
    streamSource?: StreamSourceDescription,
): [
    amplitude: Accessor<number>,
    controls: { stream: Accessor<MediaStream | undefined>; stop: () => void },
] {
    // get the stream and measure it
    const [stream, controls] = createStream(streamSource);
    const [amplitude, stopAmplitude] = createAmplitudeFromStream(stream);
    const stop = (): void => {
        stopAmplitude();
        controls.stop();
    };
    onCleanup(stop);

    return [amplitude, { stream, stop }];
}

/** Ask for media permissions by opening a stream and stopping it right away, both kinds by default. */
export async function createMediaPermissionRequest(
    source?: MediaStreamConstraints | "audio" | "video",
): Promise<void> {
    // ask for nothing on the server
    if (isServer) {
        return;
    }
    const constraints =
        source === undefined
            ? { audio: true, video: true }
            : typeof source === "string"
              ? { [source]: true }
              : source;
    stopStream(await navigator.mediaDevices.getUserMedia(constraints));
}

/** Open a stream for each source, dropping one a newer source replaced, until cleanup. */
function followStream<Source>(
    source: Accessor<Source | undefined>,
    open: (current: Source) => Promise<MediaStream>,
    stream: Accessor<MediaStream | undefined>,
    setStream: (next: MediaStream | undefined) => void,
): void {
    createEffect(
        source,
        (current) => {
            // stop the last stream, and open the next one unless replaced first
            stopStream(untrack(stream));
            setStream(undefined);
            if (current === undefined) {
                return undefined;
            }
            let isActive = true;
            open(current).then((opened) => {
                if (isActive) {
                    setStream(opened);
                } else {
                    stopStream(opened);
                }
            }, reportError);

            return () => {
                isActive = false;
            };
        },
        TRANSPARENT,
    );
    onCleanup(() => stopStream(untrack(stream)));
}

/** Make a stream's stop and mute controls. */
function controlsOf(
    stream: Accessor<MediaStream | undefined>,
    setStream: (next: MediaStream | undefined) => void,
): StreamControls {
    return {
        mute: (muted = true) => {
            for (const track of untrack(stream)?.getTracks() ?? []) {
                track.enabled = !muted;
            }
        },
        stop: () => {
            stopStream(untrack(stream));
            setStream(undefined);
        },
    };
}

/** Read the constraints of a source: a device's identifier for its kind, or the constraints as given. */
function constraintsOf(
    source: MediaDeviceInfo | MediaStreamConstraints | FalsyValue,
): MediaStreamConstraints | undefined {
    if (
        source === false ||
        source === 0 ||
        source === "" ||
        source === null ||
        source === undefined
    ) {
        return undefined;
    }

    return "deviceId" in source
        ? {
              [source.kind === "videoinput" ? "video" : "audio"]: {
                  deviceId: { exact: source.deviceId },
              },
          }
        : source;
}

/** Stop every track of a stream. */
function stopStream(stream: MediaStream | undefined): void {
    for (const track of stream?.getTracks() ?? []) {
        track.stop();
    }
}

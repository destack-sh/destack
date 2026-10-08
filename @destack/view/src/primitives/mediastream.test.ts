import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import {
    createAmplitudeFromStream,
    createMediaPermissionRequest,
    createScreen,
    createStream,
} from "./mediastream.ts";

/** A stand-in track that records being stopped and muted. */
class StubTrack {
    /** Whether the track plays, behind the interface's name. */
    isEnabled = true;
    /** Whether the track was stopped. */
    isStopped = false;

    /** Read whether the track plays, false while muted. */
    get enabled(): boolean {
        return this.isEnabled;
    }

    /** Write whether the track plays. */
    set enabled(value: boolean) {
        this.isEnabled = value;
    }

    /** Stop the track. */
    stop(): void {
        this.isStopped = true;
    }
}

/** A stand-in stream of one track, named after the constraints that opened it. */
class StubStream {
    /** The stream's one track. */
    readonly track = new StubTrack();
    /** What opened the stream. */
    readonly source: string;

    /** Name the stream. */
    constructor(source: string) {
        this.source = source;
    }

    /** List the stream's tracks. */
    getTracks(): StubTrack[] {
        return [this.track];
    }
}

/** The streams opened in the current test. */
let opened: StubStream[] = [];

beforeEach(() => {
    opened = [];
    const open = async (constraints: unknown): Promise<StubStream> => {
        const stream = new StubStream(JSON.stringify(constraints));
        opened.push(stream);

        return stream;
    };
    Object.defineProperty(navigator, "mediaDevices", {
        configurable: true,
        value: { getUserMedia: open, getDisplayMedia: open },
    });
});

afterEach(() => {
    Reflect.deleteProperty(navigator, "mediaDevices");
    vi.unstubAllGlobals();
});

/** Wait for the streams to open. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
    flush();
}

test("open a stream for each source, stopping the last, and mute and stop it", async () => {
    // open from constraints, then from a device
    const [source, setSource] = createSignal<MediaStreamConstraints | MediaDeviceInfo | undefined>({
        audio: true,
    });
    const { stream, controls, dispose } = createRoot((disposeRoot) => {
        const [followed, openedControls] = createStream(source);

        return { stream: followed, controls: openedControls, dispose: disposeRoot };
    });
    await settle();
    const first = opened[0];
    const deviceValues = {
        deviceId: "camera",
        groupId: "group",
        kind: "videoinput" as const,
        label: "camera",
    };
    setSource({ ...deviceValues, toJSON: () => deviceValues });
    await settle();

    // mute, then stop
    controls.mute();
    const isMuted = opened[1]?.track.enabled === false;
    controls.stop();
    flush();
    dispose();

    expect({
        sources: opened.map((item) => item.source),
        firstStopped: first?.track.isStopped,
        isMuted,
        secondStopped: opened[1]?.track.isStopped,
        stream: stream(),
    }).toEqual({
        sources: ['{"audio":true}', '{"video":{"deviceId":{"exact":"camera"}}}'],
        firstStopped: true,
        isMuted: true,
        secondStopped: true,
        stream: undefined,
    });
});

test("capture the screen and stop it on cleanup", async () => {
    const { stream, dispose } = createRoot((disposeRoot) => {
        const [captured] = createScreen({ video: true });

        return { stream: captured, dispose: disposeRoot };
    });
    await settle();
    const isOpen = stream() !== undefined;
    dispose();

    expect([isOpen, opened[0]?.track.isStopped]).toEqual([true, true]);
});

test("ask for both kinds of media by default, stopping the stream right away", async () => {
    await createMediaPermissionRequest();
    await createMediaPermissionRequest("audio");

    expect(opened.map((item) => [item.source, item.track.isStopped])).toEqual([
        ['{"audio":true,"video":true}', true],
        ['{"audio":true}', true],
    ]);
});

test("follow a stream's loudness each frame, capped at a hundred", () => {
    // stand in for the audio context with an analyser of fixed levels
    let level = 10;
    const states: string[] = [];
    class StubContext {
        /** Whether the context runs or was closed. */
        state = "running";
        /** The streams connected to it. */
        readonly connected: unknown[] = [];
        /** Make an analyser that reads fixed levels. */
        createAnalyser(): object {
            return {
                frequencyBinCount: 4,
                getByteFrequencyData: (bins: Uint8Array) => bins.fill(level),
            };
        }
        /** Make a source of a stream. */
        createMediaStreamSource(stream: unknown): object {
            return { connect: () => this.connected.push(stream), disconnect: () => {} };
        }
        /** Resume the context. */
        async resume(): Promise<void> {
            states.push("resumed");
        }
        /** Close the context. */
        async close(): Promise<void> {
            this.state = "closed";
            states.push("closed");
        }
    }
    vi.stubGlobal("AudioContext", StubContext);
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
        frames.push(callback),
    );
    vi.stubGlobal("cancelAnimationFrame", () => {});

    // read a quiet frame and a loud one
    const { amplitude, stop } = createRoot(() => {
        const [loudness, stopReading] = createAmplitudeFromStream(new MediaStream());

        return { amplitude: loudness, stop: stopReading };
    });
    flush();
    const quiet = amplitude();
    level = 200;
    frames.at(-1)?.(0);
    flush();
    const loud = amplitude();
    stop();

    expect({ quiet, loud, states }).toEqual({
        quiet: 40,
        loud: 100,
        states: ["resumed", "closed"],
    });
});

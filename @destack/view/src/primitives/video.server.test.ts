import { expect, test } from "@destack/test";
import {
    createVideo,
    createVideoFrameCallback,
    createVideoPlayer,
    makeVideo,
    makeVideoPlayer,
} from "./video.ts";

test("play no video on the server", () => {
    const video = createVideoPlayer("clip.mp4");

    expect([
        makeVideo("clip.mp4")[0],
        makeVideoPlayer("clip.mp4")[0].player,
        createVideo("clip.mp4").player,
        video.playing(),
        video.volume(),
        video.readyState(),
        createVideoFrameCallback(
            () => undefined,
            () => {},
        )[0](),
    ]).toEqual([undefined, undefined, undefined, false, 1, 0, false]);
});

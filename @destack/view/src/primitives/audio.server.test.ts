import { expect, test } from "@destack/test";
import { NotReadyError } from "solid-js";
import { createAudio, makeAudio, makeAudioPlayer } from "./audio.ts";

test("play nothing on the server, keeping the length pending", () => {
    const audio = createAudio("sound.mp3");
    let duration: unknown;
    try {
        duration = audio.duration();
    } catch (error) {
        duration = error instanceof NotReadyError ? "pending" : error;
    }

    expect([
        makeAudio("sound.mp3")[0],
        makeAudioPlayer("sound.mp3")[0].player,
        audio.player,
        audio.playing(),
        audio.volume(),
        duration,
    ]).toEqual([undefined, undefined, undefined, false, 1, "pending"]);
});

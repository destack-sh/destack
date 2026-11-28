import { expect, test } from "@destack/test";
import { Navigation } from "./navigation.ts";

test("read a server render's URL and refuse every move, as a render has no history", () => {
    // render at an address with a query
    const url = "https://tasks.example.test/board?label=bug";
    const rendered = Navigation.rendered(url);
    const moved = [
        () => rendered.navigate("/other"),
        () => rendered.assign("/other"),
        () => rendered.reload(),
        () => rendered.go(-1),
    ].map((move) => {
        try {
            move();

            return "moved";
        } catch (error) {
            return error instanceof Error ? error.message : "not an error";
        }
    });

    // read the entry, and refuse each move with the same blocked capability
    const refused = `cannot navigate from '${url}': a server render has no history`;
    expect([rendered.currentEntry, moved]).toEqual([
        { url, index: 0, state: undefined },
        [refused, refused, refused, refused],
    ]);
});

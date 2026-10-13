import { expect, test } from "@destack/test";
import { CallKey } from "./key.ts";

test("derive an installation's secrets alike on each of its instances and apart from another installation's", async () => {
    // derive each installation's key from one host key, twice for the first
    const host = await CallKey.import(new Uint8Array(32).fill(7));
    const derive = async (installation: string) => {
        const key = await CallKey.import(await CallKey.derive(host, installation));
        const deriver = await CallKey.deriver(key);

        return (await deriver.derive("destack watch v1 channel-1")).toHex();
    };
    const [first, again, other] = await Promise.all([
        derive("installation-a"),
        derive("installation-a"),
        derive("installation-b"),
    ]);

    expect({ isRepeated: first === again, isApart: first !== other }).toEqual({
        isRepeated: true,
        isApart: true,
    });
});

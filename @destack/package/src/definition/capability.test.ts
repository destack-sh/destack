import { expect, test } from "@destack/test";
import { Capabilities } from "./capability.ts";

/** An indexer's capabilities, each with its reason. */
const indexer = {
    process: { reason: "indexes checkouts with native tools" },
    network: {
        connect: ["api.github.com", "*.githubusercontent.com:443"],
        reason: "fetches linked repositories",
    },
    run: { commands: ["git"], reason: "reads history" },
    fs: { access: "read", reason: "indexes your notes" },
    listen: { reason: "serves its index to the editor" },
    "clipboard-write": { reason: "copies search results" },
};

test("grant the required capabilities and only the optional ones an installation allows", () => {
    // declare an optional network, directory access and camera beside a required process, commands and clipboard
    const declared = Capabilities.parse({
        ...indexer,
        network: { ...indexer.network, optional: true },
        fs: { ...indexer.fs, optional: true },
        camera: { reason: "scans receipts", optional: true },
    });
    const { network: _network, fs: _fs, camera: _camera, ...required } = declared;

    // keep the required ones without allowances, and add the optional ones under allowed names
    expect([
        Capabilities.grant(declared, []),
        Capabilities.grant(declared, ["network", "camera"]),
        Capabilities.grant(declared, ["fs", "microphone"]),
    ]).toEqual([
        required,
        { ...required, network: declared.network, camera: declared.camera },
        { ...required, fs: declared.fs },
    ]);
});

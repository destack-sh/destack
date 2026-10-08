import { expect, test } from "@destack/test";
import { writeManifest } from "./manifest.ts";

test("write the manifest members the specification spells, starting at the root by default", () => {
    expect(
        JSON.parse(
            writeManifest({
                name: "Destack",
                shortName: "Destack",
                display: "standalone",
                themeColor: "#0b2029",
                icons: [
                    {
                        src: "/icon-512.png",
                        sizes: "512x512",
                        type: "image/png",
                        purpose: "maskable",
                    },
                ],
            }),
        ),
    ).toEqual({
        name: "Destack",
        short_name: "Destack",
        start_url: "/",
        display: "standalone",
        theme_color: "#0b2029",
        icons: [{ src: "/icon-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" }],
    });
});

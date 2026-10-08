import { expect, test } from "@destack/test";
import { writeSecurity } from "./security.ts";

/** The moment the files are written at. */
const NOW = new Date("2026-10-08T12:00:00Z");

test("write the contacts, expiry, languages and policy, canonical at the well-known address", () => {
    expect(
        writeSecurity(
            "https://destack.sh",
            {
                contact: ["mailto:security@destack.sh", "https://destack.sh/security"],
                expires: "2027-06-30T00:00:00Z",
                preferredLanguages: ["en", "de"],
                policy: "https://destack.sh/docs/security",
            },
            NOW,
        ),
    ).toBe(
        [
            "Contact: mailto:security@destack.sh",
            "Contact: https://destack.sh/security",
            "Expires: 2027-06-30T00:00:00.000Z",
            "Preferred-Languages: en, de",
            "Policy: https://destack.sh/docs/security",
            "Canonical: https://destack.sh/.well-known/security.txt",
            "",
        ].join("\n"),
    );
});

test("refuse an expiry that has passed", () => {
    expect(() =>
        writeSecurity(
            "https://destack.sh",
            { contact: ["mailto:a@b.c"], expires: "2026-01-01T00:00:00Z" },
            NOW,
        ),
    ).toThrow("security.txt expires in the past or never: 2026-01-01T00:00:00Z");
});

import { expect, refusal, test } from "@destack/test";
import { writeSecurity } from "./security.ts";

/** The moment the files are written at. */
const NOW = new Date("2026-10-08T12:00:00Z");

test("write the contacts, the expiry a lifetime after now, languages and policy, canonical at the well-known address", () => {
    expect(
        writeSecurity(
            "https://destack.sh",
            {
                contact: ["mailto:security@destack.sh", "https://destack.sh/security"],
                lifetime: { days: 183 },
                preferredLanguages: ["en", "de"],
                policy: "https://destack.sh/docs/security",
            },
            NOW,
        ),
    ).toBe(
        [
            "Contact: mailto:security@destack.sh",
            "Contact: https://destack.sh/security",
            "Expires: 2027-04-09T12:00:00.000Z",
            "Preferred-Languages: en, de",
            "Policy: https://destack.sh/docs/security",
            "Canonical: https://destack.sh/.well-known/security.txt",
            "",
        ].join("\n"),
    );
});

test("refuse a lifetime of nothing or a year", async () => {
    const write = (lifetime: { days: number }) => () =>
        writeSecurity("https://destack.sh", { contact: ["mailto:a@b.c"], lifetime }, NOW);
    expect([await refusal(write({ days: 0 })), await refusal(write({ days: 365 }))]).toEqual([
        [
            "BAD_REQUEST",
            "SECURITY_LIFETIME_INVALID",
            `security.txt lifetime {"days":0} is invalid: it must be less than a year`,
        ],
        [
            "BAD_REQUEST",
            "SECURITY_LIFETIME_INVALID",
            `security.txt lifetime {"days":365} is invalid: it must be less than a year`,
        ],
    ]);
});

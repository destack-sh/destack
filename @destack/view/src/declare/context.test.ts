import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { ViewLaunch } from "./context.ts";

test("accept only endpoints that stay on the page's origin", () => {
    // resolve each endpoint as the browser does beside the launch's verdict
    const page = "https://notes.test";
    const verdicts = ["/replica", "//evil.test/replica", "/\\evil.test", "/\t/evil.test"].map(
        (endpoint) => [
            new URL(endpoint, page).origin === page,
            ViewLaunch.safeParse({
                installation: "installation-notes",
                space: "space-notes",
                account: "account-notes",
                view: "notes",
                user: principal.user.reference(Scope.universe.id, "alice"),
                release: "2026.9.0",
                endpoint,
                catalogs: [],
            }).success,
        ],
    );

    // accept only the endpoint the browser keeps on the page's origin
    expect(verdicts).toEqual([
        [true, true],
        [false, false],
        [false, false],
        [false, false],
    ]);
});

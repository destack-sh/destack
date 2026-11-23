import { expect, test } from "@destack/test";
import { GitCredentialRequest } from "./git.ts";

test("answer git's credential request with a user and the token as its password, ephemeral where git can mark it", () => {
    // read a request of a git with the authtype capability, and one of an earlier git
    const recent = GitCredentialRequest.parse(
        "capability[]=authtype\nprotocol=https\nhost=universe.test\n\n",
    );
    const earlier = GitCredentialRequest.parse("protocol=https\nhost=universe.test\n\n");

    // expect the same credential for both, marked ephemeral for the git that announced the capability
    expect([recent.answer("token-a"), earlier.answer("token-a")]).toEqual([
        "capability[]=authtype\nusername=x-access-token\npassword=token-a\nephemeral=true",
        "username=x-access-token\npassword=token-a",
    ]);
});

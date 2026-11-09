import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { Card, CardContent, CardHeader, CardTitle } from "../card/index.ts";
import { Stack } from "../stack/index.ts";
import { SignUpHandle, SignUpResidency } from "./sign-up.tsx";

/** A sign-up page whose handle is checked as it is typed, its residency suggested by country. */
export const signUpWithHandle = defineExample({
    of: SignUpHandle,
    name: "with-handle",
    description:
        "a sign-up page whose handle is checked as it is typed, its residency suggested by country",
    render: () => {
        const [handle, setHandle] = createSignal("ada");
        const [residency, setResidency] = createSignal("eu");

        return (
            <Card>
                <CardHeader>
                    <CardTitle render={(attributes) => <h1 {...attributes} />}>
                        Create your Destack
                    </CardTitle>
                </CardHeader>
                <CardContent>
                    <Stack space="3">
                        <SignUpHandle
                            value={handle()}
                            availability={handle() === "ada" ? "taken" : "available"}
                            onInput={setHandle}
                        />
                        <SignUpResidency
                            residencies={[
                                { code: "eu", name: "European Union" },
                                { code: "us", name: "United States" },
                            ]}
                            value={residency()}
                            onChange={setResidency}
                        />
                    </Stack>
                </CardContent>
            </Card>
        );
    },
});

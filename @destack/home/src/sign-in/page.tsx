import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { Alert, AlertDescription } from "@destack/ui/alert";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@destack/ui/card";
import { Center } from "@destack/ui/center";
import { Stack } from "@destack/ui/stack";
import { type JSX, Show } from "@destack/view";

/** The page's layout: one centred card as wide as a phone. */
const styles = style.create({
    card: { width: "min(100%, 24rem)", marginBlock: space[8] },
});

/** Render a page as one centred card: its title, description, last refusal and steps. */
export function CardPage(properties: {
    /** The page's heading. */
    readonly title: string;
    /** The line below the heading. */
    readonly description?: string | undefined;
    /** The refusal of the last step, shown above the steps. */
    readonly error?: string | undefined;
    /** The steps. */
    readonly children?: JSX.Element;
}) {
    return (
        <Center intrinsic>
            <Card xstyle={styles.card}>
                <CardHeader>
                    <CardTitle render={(attributes) => <h1 {...attributes} />}>
                        {properties.title}
                    </CardTitle>
                    <Show when={properties.description}>
                        {(text) => <CardDescription>{text()}</CardDescription>}
                    </Show>
                </CardHeader>
                <CardContent>
                    <Stack space="4">
                        <Show when={properties.error}>
                            {(message) => (
                                <Alert variant="destructive">
                                    <AlertDescription>{message()}</AlertDescription>
                                </Alert>
                            )}
                        </Show>
                        {properties.children}
                    </Stack>
                </CardContent>
            </Card>
        </Center>
    );
}

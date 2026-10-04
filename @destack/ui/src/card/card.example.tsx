import type { JSX } from "@solidjs/web";
import { Button } from "../button/index.ts";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "./card.tsx";

/** Show a storage plan with an upgrade action and a footer link. */
export function CardExample(): JSX.Element {
    return (
        <Card>
            <CardHeader>
                <CardTitle>Storage</CardTitle>
                <CardDescription>2 of 5 GB used</CardDescription>
                <CardAction>
                    <Button variant="outline" size="sm">
                        Upgrade
                    </Button>
                </CardAction>
            </CardHeader>
            <CardContent>Notes take 1.2 GB, files 0.6 GB and photos 0.2 GB.</CardContent>
            <CardFooter>
                <Button variant="link">Manage storage</Button>
            </CardFooter>
        </Card>
    );
}

import type { JSX } from "@solidjs/web";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs.tsx";

/** Show a note's editor and preview as tabs. */
export function TabsExample(): JSX.Element {
    return (
        <Tabs defaultValue="edit">
            <TabsList aria-label="Note">
                <TabsTrigger value="edit">Edit</TabsTrigger>
                <TabsTrigger value="preview">Preview</TabsTrigger>
            </TabsList>
            <TabsContent value="edit">The editor</TabsContent>
            <TabsContent value="preview">The rendered note</TabsContent>
        </Tabs>
    );
}

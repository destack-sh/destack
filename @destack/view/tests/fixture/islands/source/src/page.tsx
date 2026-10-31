import Counter from "./islands/counter.tsx";

/** Render the page's static heading beside its counter. */
export default function Page() {
    return (
        <main>
            <h1>Static heading</h1>
            <Counter start={3} />
        </main>
    );
}

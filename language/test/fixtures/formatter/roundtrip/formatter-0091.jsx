// None of these were declared in this file.
// It's bad to register them because that would trigger
// modules to execute in an environment with inline loaders.
// So we expect the transform to skip all of them even though
// they are used in JSX.

const A = load("A");
const B = foo ? load("X") : load("Y");
const C = loadCond(gk, "C");
const D = load("D");

export default function App() {
    return (
        <div>
            <A />
            <B />
            <C />
            <D />
        </div>
    );
}

import "./index.css";
import { Breakout } from "./game/engine";

export function App() {
  return (
    <div className="container mx-auto p-8 text-center relative z-10">
      <h1 className="text-4xl font-bold mb-6">Breakout</h1>
      <div className="grid place-items-center">
        <Breakout />
      </div>
      <p className="mt-4 text-sm text-muted-foreground">arrow keys or A/D to move • space to launch</p>
    </div>
  );
}

export default App;

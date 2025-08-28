import "../index.css";
import { Toolbar } from "./Toolbar";
import { Sidebar } from "./Sidebar";
import { Canvas } from "./Canvas";
import { Inspector } from "./Inspector";
import { Timeline } from "./Timeline";

export function App() {
  return (
    <div className="grid grid-rows-[auto_1fr_auto] grid-cols-[280px_1fr_320px] h-dvh">
      <div className="col-span-3 border-b">
        <Toolbar />
      </div>
      <div className="border-r overflow-auto">
        <Sidebar />
      </div>
      <div className="overflow-hidden">
        <Canvas />
      </div>
      <div className="border-l overflow-auto">
        <Inspector />
      </div>
      <div className="col-span-3 border-t">
        <Timeline />
      </div>
    </div>
  );
}

export default App;

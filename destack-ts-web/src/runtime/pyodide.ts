import { useEffect, useRef, useState } from "react";

export enum PyodideWorkerStatus {
  INITIALIZING = "INITIALIZING",
  READY = "READY",
  RUNNING = "RUNNING",
  ERROR = "ERROR",
}

export type PyodideRequest = { type: "init" } | { type: "run"; data: { code: string } };

export type PyodideResponse =
  | { type: "status"; message: PyodideWorkerStatus }
  | { type: "result"; result: string | null; stdout: string; duration: number }
  | { type: "error"; message: string; duration: number };

export function usePyodideWorker(): {
  status: PyodideWorkerStatus;
  output: string;
  duration: number | null;
  isRunning: boolean;
  runCode: (code: string) => void;
} {
  const workerRef = useRef<Worker | null>(null);

  const [status, setStatus] = useState<PyodideWorkerStatus>(PyodideWorkerStatus.INITIALIZING);
  const [output, setOutput] = useState<string>("");
  const [duration, setDuration] = useState<number | null>(null);
  const [isRunning, setIsRunning] = useState<boolean>(false);

  // initialize web worker
  useEffect(() => {
    workerRef.current = new Worker("/pyodide.js");

    workerRef.current.onmessage = (event) => {
      const data = event.data as PyodideResponse;

      switch (data.type) {
        case "status":
          setStatus(data.message);
          break;
        case "result":
          setIsRunning(false);
          const resultOutput = data.stdout
            ? `${data.stdout}${data.result ? `\n${data.result}` : ""}`
            : data.result || "";
          setOutput(resultOutput);
          setDuration(data.duration);
          break;
        case "error":
          setIsRunning(false);
          setOutput(`Error: ${data.message}`);
          setDuration(data.duration);
          break;
      }
    };

    workerRef.current.onerror = (error) => {
      setStatus(PyodideWorkerStatus.ERROR);
      setOutput(`Worker Error: ${error.message}`);
      setDuration(null);
    };

    // initialize Pyodide in the worker
    const initRequest: PyodideRequest = { type: "init" };
    workerRef.current.postMessage(initRequest);

    return () => {
      workerRef.current?.terminate();
      workerRef.current = null;
    };
  }, []);

  const runCode = (code: string): void => {
    if (status !== PyodideWorkerStatus.READY || !workerRef.current) return;

    setIsRunning(true);
    setOutput("");
    setDuration(null);

    const runRequest: PyodideRequest = {
      type: "run",
      data: { code },
    };

    workerRef.current.postMessage(runRequest);
  };

  return {
    status,
    output,
    duration,
    isRunning,
    runCode,
  };
}

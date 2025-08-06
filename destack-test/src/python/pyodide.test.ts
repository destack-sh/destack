import { expect, test } from "bun:test";
import { loadDestackPython, loadPython } from "@destack-test/python/pyodide";
import { VERSION } from "destack";

test("loadPython", async () => {
  const pyodide = await loadPython();
  expect(pyodide).toBeDefined();

  const result = await pyodide.runPython(`2 + 2`);
  expect(result).toBe(4);
});

test("loadDestackPython", async () => {
  const python = await loadDestackPython();
  expect(python).toBeDefined();

  const result = await python.runPython(`VERSION`);
  expect(result).toBe(VERSION);
});

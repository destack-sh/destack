import { loadPyodide, type PyodideInterface } from "pyodide";

const DESTACK_PY_PATH = new URL("../../../destack-py/destack", import.meta.url).pathname;

/** Load the Python interpreter. */
export async function loadPython(): Promise<PyodideInterface> {
  const pyodide = await loadPyodide();
  return pyodide;
}

/** Load the Destack Python SDK. */
export async function loadDestackPython(): Promise<PyodideInterface> {
  const python = await loadPython();

  // load dependencies
  const requirementsFile = Bun.file(`${DESTACK_PY_PATH}/requirements/requirements.in`);
  const requirementsText = await requirementsFile.text();
  const requirements = requirementsText
    .split("\n")
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => line.split("==")[0].trim());
  for (const requirement of requirements) {
    await python.loadPackage(requirement);
  }

  // load destack python SDK
  python.mountNodeFS("/home/pyodide/destack", DESTACK_PY_PATH);
  await python.runPythonAsync(`
		from destack import *
		print(VERSION)
	`);
  return python;
}

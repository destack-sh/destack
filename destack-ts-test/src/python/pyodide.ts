import { loadPyodide, type PyodideInterface } from "pyodide";

// TODO: generalize pyodide for destack-ts-*? (core?/web/system/simulation)
//  or maybe just drop pyodide on web for now? (would simplify the destack-py protection requirements)

const DESTACK_ROOT_PATH = new URL("../../../", import.meta.url).pathname;
const DESTACK_ENV_PATHS = [".env.test", ".env.test.local"];
const DESTACK_PY_PATH = new URL(`${DESTACK_ROOT_PATH}/destack-py/destack`, import.meta.url)
  .pathname;

/** Get environment variables from the root of the project. */
async function getEnv(): Promise<{ [key: string]: string }> {
  const env: { [key: string]: string } = {};
  for (const envPath of DESTACK_ENV_PATHS) {
    const envFile = Bun.file(`${DESTACK_ROOT_PATH}/${envPath}`);
    const envText = await envFile.text();
    for (const line of envText.split("\n")) {
      const [key, value] = line.split("=");
      env[key] = value;
    }
  }
  return env;
}

/** Get the requirements for the Destack Python SDK. */
async function getRequirements(): Promise<string[]> {
  const requirementsFile = Bun.file(`${DESTACK_PY_PATH}/requirements/requirements.in`);
  const requirementsText = await requirementsFile.text();
  const requirements = requirementsText
    .split("\n")
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => line.split("==")[0].trim());
  return ["tzdata", ...requirements];
}

/** Load the Python interpreter. */
export async function loadPython(options?: { packages?: string[] }): Promise<PyodideInterface> {
  // load environment variables from the root of the project (manually is more robust)
  const testEnv = await getEnv();
  const pyodide = await loadPyodide({
    env: testEnv,
    packages: options?.packages ?? [],
  });
  return pyodide;
}

/** Load the Destack Python SDK. */
export async function loadDestackPython(): Promise<PyodideInterface> {
  const requirements = await getRequirements();
  const python = await loadPython({ packages: requirements });
  // load destack python SDK
  python.mountNodeFS("/home/pyodide/destack", DESTACK_PY_PATH);
  await python.runPythonAsync(`
		from destack import *
	`);
  return python;
}

import { expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { RemoteWorkspaceServer } from "@destack/language-napi";
import {
  memoryFiles,
  memoryRoot,
} from "../dist/workspace/memory.js";
import {
  openWorkspace,
  type Json,
  type Workspace,
} from "../dist/index.js";

/** One client workspace test project. */
type Project = {
  /** Typed `destack.json` content. */
  readonly config: Json;
  /** Repository relative text files. */
  readonly files: Readonly<Record<string, string>>;
};

/** One opened test workspace. */
type OpenedWorkspace = {
  /** The workspace under test. */
  readonly workspace: Workspace;
  /** Close the workspace and release owned resources. */
  close(): Promise<void> | void;
};

/** One workspace backend used by integration tests. */
type TestWorkspace = {
  /** Human-readable backend name. */
  readonly name: string;
  /** Open one project with this backend. */
  open(project: Project): Promise<OpenedWorkspace>;
};

for (const workspace of workspaces()) {
  test(`check accepts a clean ${workspace.name} workspace`, async () => {
    const opened = await workspace.open({
      config: {
        name: "@test/app",
      },
      files: {},
    });

    try {
      // run the real check command through the opened workspace
      const output = await opened.workspace.check({
        inputs: [
          {
            kind: "inline",
            name: "input.ds",
            content: "export const value = 1;\n",
            fileType: "destack",
          },
        ],
      });

      expect(output.success).toBe(true);
      expect(output.diagnostics).toEqual([]);
    } finally {
      await opened.close();
    }
  });
}

for (const workspace of workspaces()) {
  test(`format returns formatted content from a ${workspace.name} workspace`, async () => {
    const opened = await workspace.open({
      config: {
        name: "@test/app",
      },
      files: {},
    });

    try {
      // store content through the same workspace used by the formatter
      const content = await opened.workspace.store("export  const value=1;\n");

      // format the stored content through the opened workspace
      const output = await opened.workspace.format({
        source: {
          content,
          fileType: "destack",
          name: "input.ds",
        },
      });

      expect(output.success).toBe(true);
      expect(output.data.formatted).toBe("export const value = 1;\n");
    } finally {
      await opened.close();
    }
  });
}

test("memory workspace inputs normalize config and files", () => {
  const workspace = {
    memory: {
      root: "/project",
      config: {
        name: "@test/app",
      },
      files: {
        "src/index.ds": "export const value = 1;\n",
        "asset.bin": [1, 2, 3],
      },
    },
  };

  // build the exact file payload sent to local native workspace servers
  const files = memoryFiles(workspace);

  expect(memoryRoot(workspace)).toBe("/project");
  expect(files).toEqual([
    {
      path: "destack.json",
      text: "{\"name\":\"@test/app\"}\n",
    },
    {
      path: "src/index.ds",
      text: "export const value = 1;\n",
    },
    {
      path: "asset.bin",
      bytes: new Uint8Array([1, 2, 3]),
    },
  ]);
});

test("memory workspace inputs reject ambiguous files", () => {
  const workspace = {
    memory: {
      files: [
        {
          path: "src/index.ds",
          text: "export const value = 1;\n",
          bytes: new Uint8Array([1]),
        },
      ],
    },
  };

  // a file cannot be both text and bytes
  expect(() => memoryFiles(workspace)).toThrow("text and bytes");
});

/** Return client workspaces used by integration tests. */
function workspaces(): readonly TestWorkspace[] {
  return [
    {
      name: "local",
      open: openLocalWorkspace,
    },
    {
      name: "remote",
      open: openRemoteWorkspace,
    },
  ];
}

/** Open one embedded local workspace. */
async function openLocalWorkspace(project: Project): Promise<OpenedWorkspace> {
  const workspace = await openWorkspace({
    memory: {
      config: project.config,
      files: project.files,
    },
  });

  return {
    workspace,
    close: () => workspace.close(),
  };
}

/** Open one remote workspace through an in-process protocol server. */
async function openRemoteWorkspace(project: Project): Promise<OpenedWorkspace> {
  const root = writeProject(project);
  const server = RemoteWorkspaceServer.open(root);

  try {
    const workspace = await openWorkspace({
      url: server.url(),
      workspace: root,
    });

    return {
      workspace,
      close: async () => {
        try {
          await workspace.close();
        } finally {
          server.close();
          rmSync(root, { recursive: true, force: true });
        }
      },
    };
  } catch (error) {
    server.close();
    rmSync(root, { recursive: true, force: true });
    throw error;
  }
}

/** Write one project tree to a temporary directory. */
function writeProject(project: Project): string {
  const root = mkdtempSync(join(tmpdir(), "destack-client-project-"));
  const config = `${JSON.stringify(project.config)}\n`;
  writeFileSync(join(root, "destack.json"), config, "utf8");

  for (const [path, text] of Object.entries(project.files)) {
    const file = join(root, path);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, text);
  }

  return root;
}

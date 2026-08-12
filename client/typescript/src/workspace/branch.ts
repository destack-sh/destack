import type { Revision } from "../_generated/repository/revision.js";
import type { FileSelection } from "../_generated/workspace/branch.js";
import type { EditBranchRequest } from "../_generated/workspace/service/source.js";
import type { Workspace } from "./workspace.js";

/** One named workspace revision. */
export class Branch {
    /** Workspace containing this branch. */
    readonly workspace: Workspace;
    /** Branch name. */
    readonly name: string;

    /** Bind one workspace branch. */
    constructor(workspace: Workspace, name: string) {
        this.workspace = workspace;
        this.name = name;
    }

    /** Read this branch's current revision. */
    async revision(): Promise<Revision> {
        const response = await this.workspace.client.branchRevision({
            root: this.workspace.root,
            name: this.name,
        });

        return response.value;
    }

    /** Commit source edits to one exact branch revision. */
    async edit(request: Omit<EditBranchRequest, "root" | "name">) {
        const response = await this.workspace.client.editBranch({
            root: this.workspace.root,
            name: this.name,
            ...request,
        });

        return response.value;
    }

    /** Save selected files to physical workspace state. */
    async save(files: FileSelection) {
        const [revision, physical] = await Promise.all([
            this.revision(),
            this.workspace.revision(),
        ]);

        const response = await this.workspace.client.saveBranch({
            root: this.workspace.root,
            name: this.name,
            revision,
            physical,
            files,
        });

        return response.value;
    }

    /** Restore selected files from physical workspace state. */
    async restore(files: FileSelection) {
        const [revision, physical] = await Promise.all([
            this.revision(),
            this.workspace.revision(),
        ]);

        const response = await this.workspace.client.restoreBranch({
            root: this.workspace.root,
            name: this.name,
            revision,
            physical,
            files,
        });

        return response.value;
    }

    /** Watch this branch for exact committed transitions. */
    watch() {
        return this.workspace.client.watchBranch({
            root: this.workspace.root,
            name: this.name,
        });
    }

    /** Remove this branch. */
    async remove(): Promise<void> {
        await this.workspace.client.removeBranch({
            root: this.workspace.root,
            name: this.name,
        });
    }
}

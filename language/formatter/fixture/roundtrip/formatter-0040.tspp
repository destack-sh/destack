import fs from "fs";
import os from "os";
import path from "path";

const benchTitle = "Page Load Tests";
const workDir = fs.mkdtempSync(path.join(os.tmpdir(), "next-stats"));
const mainRepoDir = path.join(workDir, "main-repo");
const diffRepoDir = path.join(workDir, "diff-repo");
const statsAppDir = path.join(workDir, "stats-app");
const diffingDir = path.join(workDir, "diff");
const allowedConfigLocations = ["./", ".stats-app", "test/.stats-app", ".github/.stats-app"];

export default {
    benchTitle,
    workDir,
    diffingDir,
    mainRepoDir,
    diffRepoDir,
    statsAppDir,
    allowedConfigLocations,
};

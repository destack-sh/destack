#!/usr/bin/env node

import { runBinaryCommand } from "@destack/cli/launcher";

runBinaryCommand("destack", ["init", ...process.argv.slice(2)]);

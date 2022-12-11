import { useFileOps } from "@/utils/actions/file";
import { useSymbolOps } from "@/utils/actions/symbol";
import { useCompilationOps } from "@/utils/actions/compilation";
import { useProjectVersionOps } from "@/utils/actions/version";

export function useActions() {
  return {
    file: useFileOps(),
    symbol: useSymbolOps(),
    compilation: useCompilationOps(),
    version: useProjectVersionOps(),
  };
}

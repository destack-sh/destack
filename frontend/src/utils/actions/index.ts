import { useFileOps } from "@/utils/actions/file";
import { useSymbolOps } from "@/utils/actions/symbol";

export function useActions() {
  return {
    file: useFileOps(),
    symbol: useSymbolOps(),
  };
}

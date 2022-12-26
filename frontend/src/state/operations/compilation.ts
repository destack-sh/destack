import type { AddCompilationInput } from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";

export function useCompilationOps() {
  const operations = useOperationsStore();

  async function create(input: AddCompilationInput) {
    throw new Error("Not implemented");
  }

  async function compile(id: string) {
    throw new Error("Not implemented");
  }

  return { create, compile };
}

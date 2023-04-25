import { graphql } from "@/gql";
import { useAuth } from "@/state/auth";
import { useEditorState } from "@/state/editor";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { toRef, watchEffect } from "vue";

export function newClientId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`Client:${nodeId}`);
}

export function getOrCreateClientId(): string {
  /* Gets client id from local storage or creates a new one */
  const key = "client_id";
  const existing = localStorage.getItem(key);
  if (existing) {
    return existing;
  } else {
    const newId = newClientId();
    localStorage.setItem(key, newId);
    return newId;
  }
}

function _useClient() {
  const editor = useEditorState();
  const projectVersionId = toRef(editor, "currentProjectVersionId");
  const auth = useAuth();

  const operations = useOperationsStore();

  // upsert client if logged in whenever project version changes
  // nocheckin do this
}

export const useClient = createSharedComposable(_useClient);

function _useProjectSync() {
  const editor = useEditorState();
  const projectVersionId = toRef(editor, "currentProjectVersionId");
  // TODO @Incomplete: implement basic sync
}

export const useProjectSync = createSharedComposable(_useProjectSync);

function uuidv4() {
  throw new Error("Function not implemented.");
}

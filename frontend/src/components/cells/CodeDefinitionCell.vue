<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import FunctionTypeCell from "@/components/cells/FunctionTypeCell.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useStatementContext } from "@/components/statement";
import { useTimeFromNow } from "@/composables/useNow";
import { useEditorState } from "@/state/editor";
import { useExecutions } from "@/state/executions";
import { computed, toRef, ref, type Ref } from "vue";
import { useSymbolOps } from "@/state/runtime";
import { ExecutionStatus, ExecutionTriggerType, type SimpleType } from "@/gql/graphql";

const context = useStatementContext();

const symbolOps = useSymbolOps();
const code: Ref<string> = ref(context.statement.value.code ?? "");
const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
context.syncCode(
  code,
  computed(() => monacoRef.value?.focused)
);

const now = useTimeFromNow();

// TODO @Broken @Performance @UX: load inline code executions more sensibly
const editor = useEditorState();
const executions = useExecutions(
  {
    projectId: toRef(editor, "currentProjectId"),
    projectVersionId: toRef(editor, "currentProjectVersionId"),
    codeIds: ref([context.statement.value.id]),
    buildIds: ref(null),
    includeAncestorVersions: ref(false),
    taskIds: ref(null),
  },
  { root: true, limit: 3, live: true }
);
const lastExecution = computed(() => executions.executions.value[0]);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionTypeCell> | null> = ref(null);
const addingTypes = ref(false);

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    typeRef.value?.blur();
    monacoRef.value?.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    class="inline-flex"
    @navigate-down="monacoRef?.focus"
    @navigate-right="typeRef?.focus"
  />
  <!-- Inline type -->
  <button
    v-if="!context.readonly.value && context.typeNodes.value.length == 0"
    ref="typeRef"
    class="z-10 ml-2 w-fit rounded-sm px-0.5 text-sm hover:bg-orange-100 hover:text-gray-700"
    :class="context.focused.value ? 'text-gray-400' : 'text-gray-300'"
    @click="addingTypes = !addingTypes"
  >
    {{ addingTypes ? "-arguments" : "+arguments" }}
  </button>
  <FunctionTypeCell
    v-if="context.typeNodes.value.length > 0 || addingTypes"
    ref="typeRef"
    class="py-1"
    @navigate-up="context.navigateUp"
    @navigate-down="monacoRef?.focus"
    @navigate-right="monacoRef?.focus"
    @navigate-left="declarationRef?.focus"
  />
  <!-- Code -->
  <!-- TODO @UX: figure out nicer styling for code -->
  <MonacoEditor
    ref="monacoRef"
    hide-line-numbers
    :lineNumberOffset="0"
    :line-number-shift-px="context.xOffset.value - 20"
    v-model="code"
    @navigate-up="declarationRef?.focus"
    @navigate-down="context.navigateDown"
    @navigate-left="typeRef?.focus"
    @escape="context.escape"
    @enter="context.insertBelow"
    @execute="symbolOps.run(context.statement.value)"
    language="python"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    class="-mx-1 mt-1 rounded-sm bg-gray-100 px-1 pb-1.5 pt-1"
  />
  <button
    v-if="code.trim().length == 0"
    class="absolute bottom-3 z-10 w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
    @click="monacoRef?.focus()"
  >
    +code
  </button>
  <!-- Last output/error (if any) -->
  <div
    v-if="lastExecution && lastExecution.status != ExecutionStatus.Completed"
    class="relative -mx-1 mb-0.5 w-full rounded-sm border-t border-gray-200 bg-gray-100 px-1 py-1.5 font-mono"
    :class="lastExecution.status == ExecutionStatus.Failed ? 'text-red-600' : 'text-gray-600'"
    :key="lastExecution?.id"
  >
    {{ context.statement.value?.name }} {{ lastExecution.status.toLowerCase() }}:
    <span class="font-bold">{{ lastExecution.error?.message }}</span>
    <ul class="flex flex-col">
      <li v-for="(frame, i) of lastExecution.error?.traceback" :key="i" class="flex flex-col">
        <span> {{ frame.filename }}:{{ frame.lineno }} {{ frame.name }} </span>
        <span class="ml-2"> > {{ frame.line }} </span>
      </li>
    </ul>
    <span class="absolute right-2 top-0"> ({{ now.getTimeFromNowString(lastExecution.updatedAt) }})</span>
  </div>
</template>

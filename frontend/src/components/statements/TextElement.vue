<script lang="ts" setup>
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useElementRefs } from "@/composables/useGrid";
import { StatementType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { getStatementIconSolid } from "@/state/statement";
import { syncProperty } from "@/utils/sync";
import { AtSymbolIcon } from "@heroicons/vue/24/solid";
import { computed, nextTick, ref, watch, type Ref, type Component } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const textRef: Ref<InstanceType<typeof AnnotatedText> | null> = ref(null);
const text: Ref<string> = ref(props.statement.text ?? "");
const textSync = syncProperty({
  read: () => (text.value = props.statement.text ?? ""),
  write: () => ops.symbol.updateStatementText(null, props.statement.id, props.statement.text ?? "", text.value),
  debounceMs: 500,
  debounceMaxWait: 3000,
});

function focus(position: "first" | "last" = "first") {
  textRef.value?.focus("last"); // we always want to focus the end of the text
}

function blur() {
  textRef.value?.blur();
  quickActionsRefs.refs.value.forEach((ref) => ref?.blur());
}

function onInput() {
  textSync.onLocalWrite();
  // if text statement: morph to blank if empty
  // (only if we are the ones who caused the change)
  if (
    props.focused &&
    !props.readonly &&
    props.statement.type == StatementType.Text &&
    (props.statement.headingLevel ?? 0) == 0 &&
    text.value == ""
  ) {
    // should ideally be done in one tx, but we don't have that yet
    textSync.flushNow();
    ops.statement.morph(null, props.statement.id, props.statement, {
      type: StatementType.Blank,
      headingLevel: null,
    });
  }
}

// quick inline actions
type QuickAction = {
  id: string;
  label: string;
  icon: Component;
  action: () => void;
};
const quickActions = computed(() => {
  if (
    !props.focused ||
    props.statement.type != StatementType.Text ||
    (props.statement.headingLevel ?? 0) != 0 ||
    props.statement.name != null ||
    props.readonly ||
    textRef.value?.open
  )
    return [];
  const actions: QuickAction[] = [];
  if (props.statement.name == null) {
    actions.push({
      id: "name",
      label: "Add name",
      icon: AtSymbolIcon,
      action: () => {
        ops.statement.rename(null, props.statement.id, null, "");
        nextTick(() => emit("focus", "declaration"));
      },
    });
  }
  actions.push({
    id: "task",
    label: "Turn into task",
    icon: getStatementIconSolid(StatementType.Task),
    action: () => ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Task }),
  });
  actions.push({
    id: "code",
    label: "Turn into code",
    icon: getStatementIconSolid(StatementType.Code),
    action: () => {
      ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Code });
      nextTick(() => emit("focus", "code"));
    },
  });
  actions.push({
    id: "data",
    label: "Turn into dataset",
    icon: getStatementIconSolid(StatementType.Dataset),
    action: () => {
      ops.statement.morph(null, props.statement.id, props.statement, { type: StatementType.Dataset });
      nextTick(() => emit("focus", "dataset"));
    },
  });

  return actions;
});
const quickActionsRefs = useElementRefs<HTMLButtonElement>(quickActions, {
  navigateLeft: () => textRef.value?.focus("last"),
  navigateRight: () => focusAction(0),
});

function focusAction(id: string | number) {
  quickActionsRefs.focus(id);
}

defineExpose({
  focus,
  blur,
  syncNow: () => textSync.flushNow(),
});
</script>
<template>
  <div class="relative w-full text-gray-900" @click="textRef?.focusIfUnfocused">
    <!-- Actual text -->
    <AnnotatedText
      ref="textRef"
      :model-value="text || ''"
      @update:model-value="(text = $event), onInput()"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="quickActions.length > 0 ? quickActionsRefs.focus(quickActions[0].id) : emit('navigateRight')"
      @enter-left="emit('enterLeft')"
      @enter="emit('enter')"
      @enter-right="emit('enterRight')"
      @delete-left="emit('deleteLeft')"
      @delete-if-empty="emit('deleteSelf')"
      @paste="emit('paste')"
      :focused="focused"
      :readonly="readonly"
      :statement="statement"
    />
    <!-- Placeholder if empty -->
    <template v-if="text.length == 0">&nbsp;</template>
    <button
      v-if="text.length == 0"
      class="absolute left-0 top-0 -m-0.5 -mx-0.5 flex flex-row items-center rounded-sm p-0.5 transition-colors duration-75 hover:bg-orange-100"
      :class="[focused ? 'text-gray-400' : 'text-gray-300']"
      @click="textRef?.focus"
    >
      Enter text...
    </button>
    <!-- Quick inline actions (positioned as not to disturb the flow) -->
    <!-- TODO @UX: inline actions don't wrap properly when text overflows -->
    <div v-if="quickActions.length > 0 && focused && editing" class="relative inline-block">
      <div
        class="absolute -top-[15px] z-20 ml-3 flex animate-fadein-1500 flex-row gap-1.5 whitespace-nowrap transition-opacity"
      >
        <button
          v-for="action in quickActions"
          :key="action.id"
          :ref="(ref: any) => quickActionsRefs.registerRef(action.id, ref)"
          class="group relative flex h-fit max-h-fit flex-row items-center rounded-sm px-1 py-0.5 transition-colors duration-150 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
          @click="action.action"
          @keydown.right.stop.prevent="quickActionsRefs.navigateRight(action.id)"
          @keydown.left.stop.prevent="quickActionsRefs.navigateLeft(action.id)"
        >
          <component
            :is="action.icon"
            class="h-4 w-4 text-gray-300 transition-colors duration-150 group-hover:text-gray-500 group-focus:text-gray-500"
          />
          <!-- <span class="ml-2">{{ action.id }}</span> -->
          <!-- Label popover -->
          <span
            class="pointer-events-none absolute -left-1/2 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-700 opacity-0 transition duration-150 group-focus-within:opacity-100 group-hover:opacity-100"
          >
            {{ action.label }}
          </span>
        </button>
      </div>
    </div>
  </div>
</template>

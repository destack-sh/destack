<script lang="ts" setup>
import BlankCell from "@/components/cells/BlankCell.vue";
import CommentCell from "@/components/cells/CommentCell.vue";
import { STATEMENT_CONTEXT, type StatementContext } from "@/components/statement";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { localErrorsOf } from "@/state/runtime";
import { onClickOutside, useFocusWithin, whenever } from "@vueuse/core";
import { computed, provide, ref, toRef, watch, watchEffect, type Component, type Ref } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  reference: FragmentType<typeof StatementHeaderType> | null;
  depth: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  lineNumberBase: number;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const depthOffsetX = computed(() => props.depth * 20);

const editor = useEditorState();

const isFocused = computed(() => editor.focusedElementId == statement.value?.id);
const isEditing = computed(() => isFocused.value && editor.editingElement);
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented);

// manage cells
const context: Ref<StatementContext> = computed(() => ({
  readonly: editor.readonly,
  focused: isFocused.value,
  editing: isEditing.value,
  depth: props.depth,
  xOffset: depthOffsetX.value,
  lineNumberBase: props.lineNumberBase,
  statement: props.statement,
  reference: props.reference,
  file: props.file,
}));
provide(STATEMENT_CONTEXT, context);

type Cell = {
  component: Component;
  props?: any;
};

const rootCell: Ref<Cell> = computed(() => {
  if (statement.value.type == StatementType.Comment) {
    return {
      component: CommentCell,
    };
  }

  // default to blank cell
  return {
    component: BlankCell,
    props: { showDots: true },
  };
});
const rootCellRef = ref<InstanceType<typeof BlankCell>>();

// forward focus / editing state

const containerRef = ref<HTMLElement | null>(null);
const { focused: containerFocused } = useFocusWithin(containerRef);

// focus root cell if focused in editor but not in container
watchEffect(() => {
  if (isFocused.value && !containerFocused.value) {
    rootCellRef.value?.focus();
  }
});

// defocus root cell if focused in container but not in editor
watchEffect(() => {
  if (!isFocused.value && containerFocused.value) {
    rootCellRef.value?.defocus();
  }
});

// cancel focus if clicked outside
onClickOutside(containerRef, () => {
  if (isFocused.value && containerFocused.value) {
    rootCellRef.value?.defocus();
    editor.defocusElement(statement.value as StatementHeader);
  }
});

// if anything inside the container is focused, enable editing mode
whenever(containerFocused, () => {
  if (!isEditing.value) {
    focusInEditor();
    editor.editElement(statement.value as StatementHeader);
  }
});

function focusInEditor() {
  editor.focusFile(file.value);
  editor.focusElement(statement.value as StatementHeader);
}

function onClickContainer() {
  if (!isFocused.value) {
    focusInEditor();
    editor.editElement(statement.value as StatementHeader);
  }
}

// auto scroll into focus once the element is focused if outside of viewport
// (this isn't great because it always scrolls and doesn't consider the container size)
watch(
  () => isFocused.value,
  (isFocused) => {
    if (isFocused && containerRef.value) {
      containerRef.value.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  }
);

// errors
const localErrors = localErrorsOf(statement);
const hasLocalErrors = computed(() => (localErrors.value?.length ?? 0) > 0);
</script>
<template>
  <div
    ref="containerRef"
    class="group relative min-h-[30px] border-x-0 border-gray-200 transition-colors"
    :class="{
      // 'border-gray-200 ': !isFocused,
      // 'border-l-orange-500': isFamilyFocused,
      'hover:border-l-orange-300': !isFocused,
      'pb-0.5': true,
      'font-mono': !isComment, // not sure if everything should be mono, but it's more consistent..
      italic: isCommented,
    }"
    :style="{ paddingLeft: depthOffsetX + 'px' }"
    @click="onClickContainer"
  >
    <!-- Commented overlay -->
    <div v-if="isCommented" class="absolute inset-0 z-20 bg-gray-100 opacity-50" />
    <!-- Monaco-like line numbers on the left margin -->
    <span
      class="absolute top-[3px] w-6 select-none text-right font-mono text-sm not-italic"
      :style="{ left: -30 + 'px' }"
      :class="{
        'text-orange-200': !isFocused && !(isComment || isCommented),
        'text-gray-200': !isFocused && (isComment || isCommented),
        'font-bold text-orange-600': isFocused && !(isComment || isCommented),
        'font-bold text-gray-400': isFocused && (isComment || isCommented),
      }"
      >{{ lineNumberBase + 1 }}</span
    >
    <!-- Gutter indicators on the right margin -->
    <span
      class="absolute right-0 top-[6px] select-none text-left text-sm font-bold not-italic"
      :style="{ right: -12 + 'px' }"
    >
      <span class="text-red-700" v-if="hasLocalErrors">
        {{ localErrors?.length }}
      </span>
    </span>
    <!-- Statement focus indicator (left side if not editing) -->
    <div
      class="absolute top-0 left-0 h-full w-1"
      :class="isFocused && !isEditing ? 'bg-orange-100' : 'bg-transparent'"
    />
    <!-- Statement focus indicator (top and bottom if editing) -->
    <div class="absolute top-0 left-0 h-0.5 w-full" :class="isEditing ? 'bg-orange-100' : 'bg-transparent'" />
    <div class="absolute bottom-0 left-0 h-0.5 w-full" :class="isEditing ? 'bg-orange-100' : 'bg-transparent'" />
    <!-- Main cell -->
    <div class="py-1 px-1 text-sm">
      <component ref="rootCellRef" :is="rootCell.component" v-bind="rootCell.props" />
    </div>
    <!-- Debug info -->
    <div v-if="editor.debug" class="absolute top-2 -right-1 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
      <template v-if="isFocused">f</template>
      <template v-if="isEditing">e</template>
      <template v-if="isFirstInGroup">[</template>
      <template v-if="isLastInGroup">]</template>
      <template v-if="isCommented">#</template>
      <span class="lowercase">
        {{ statement.modifier }}
        {{ statement.type }}
        <template v-if="statement.symbolType">{{ statement.symbolType }}:</template>
      </span>
      <template v-if="statement.name != null">{{ statement.name }}</template>
      r:{{ statement.revision }} i:{{ statement.orderKey }} d:{{ depth }}
    </div>
  </div>
</template>

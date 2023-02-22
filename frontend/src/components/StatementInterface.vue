<script lang="ts" setup>
import CodeDefinitionCell from "@/components/cells/CodeDefinitionCell.vue";
import CommentCell from "@/components/cells/CommentCell.vue";
import DataDefinitionCell from "@/components/cells/DataDefinitionCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import ProtoCell from "@/components/cells/ProtoCell.vue";
import TaskDefinitionCell from "@/components/cells/TaskDefinitionCell.vue";
import TypeDefinitionCell from "@/components/cells/TypeDefinitionCell.vue";
import { STATEMENT_CONTEXT, type StatementContext } from "@/components/statement";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { localErrorsOf, symbolOf } from "@/state/runtime";
import { onClickOutside, useFocusWithin, whenever } from "@vueuse/core";
import { computed, nextTick, provide, ref, watch, type Component, type Ref } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
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
const isCommentish = computed(
  () => isComment.value || isCommented.value || statement.value.type == StatementType.Blank
);

// manage cells
const context: Ref<StatementContext> = computed(() => ({
  readonly: editor.readonly,
  focused: isFocused.value,
  editing: isEditing.value,
  depth: props.depth,
  xOffset: depthOffsetX.value,
  lineNumberBase: props.lineNumberBase,
  statement: props.statement,
  reference: symbolOf(statement.value.reference?.id) ?? null,
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
  } else if (statement.value.type == StatementType.Definition) {
    if (statement.value.symbolType == SymbolType.Type) {
      return {
        component: TypeDefinitionCell,
      };
    } else if (statement.value.symbolType == SymbolType.Task) {
      return {
        component: TaskDefinitionCell,
        props: { isTyped: true },
      };
    } else if (statement.value.symbolType == SymbolType.Expectation) {
      return {
        component: TaskDefinitionCell,
        props: { isTyped: false },
      };
    } else if (statement.value.symbolType == SymbolType.Code) {
      return {
        component: CodeDefinitionCell,
      };
    } else if (statement.value.symbolType == SymbolType.Data) {
      return {
        component: DataDefinitionCell,
      };
    }

    // default to just declaration cell
    return {
      component: DeclarationCell,
    };
  } else if (statement.value.type == StatementType.Reference) {
    return {
      component: DeclarationCell,
    };
  }

  // default to empty cell
  return {
    component: ProtoCell,
    props: { showDots: true },
  };
});
const rootCellRef = ref<InstanceType<typeof ProtoCell>>();

// forward focus / editing state

const containerRef = ref<HTMLElement | null>(null);
const { focused: containerFocused } = useFocusWithin(containerRef);

// refocus if root cell changed and we're editing
watch(
  () => rootCell.value.component,
  (oldComponent, newComponent) => {
    if (isEditing.value && oldComponent !== newComponent) {
      nextTick(() => rootCellRef.value?.focus());
    }
  },
  { deep: false }
);

// focus root cell if editing in editor but not in container
whenever(isEditing, () => {
  if (isEditing.value && !containerFocused.value) {
    rootCellRef.value?.focus();
  }
});

// blur root cell if focused in container but no longer editing (or focused)
watch(
  () => [isEditing.value, containerFocused.value],
  () => {
    if (!isEditing.value && containerFocused.value) {
      rootCellRef.value?.blur();
    }
  }
);

// cancel focus if clicked outside
onClickOutside(containerRef, () => {
  if (isFocused.value) {
    rootCellRef.value?.blur();
    editor.blurElement(statement.value as StatementHeader);
  }
});

// if anything inside the container becomes focused, enable editing mode
whenever(containerFocused, () => {
  if (!isFocused.value) {
    focusInEditor();
  }
  if (!isEditing.value) {
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
  if (!containerFocused.value) {
    rootCellRef.value?.focus();
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

const actions = useActions();
</script>
<template>
  <div
    ref="containerRef"
    class="group/statement relative min-h-[30px] border-x-0 border-gray-200 transition-colors"
    :class="{
      // 'border-gray-200 ': !isFocused,
      // 'border-l-orange-500': isFamilyFocused,
      'hover:border-l-orange-300': !isFocused,
      'pb-0.5': true,
      'font-mono': editor.fontMono && !isComment, // not sure if everything should be mono, but it's more consistent..
      'text-gray-700': isCommented,
    }"
    :style="{ paddingLeft: depthOffsetX + 'px' }"
    @click="onClickContainer"
  >
    <!-- TODO @UX: focus on @mousedown would be more responsive but doesn't focus properly.. -->
    <!-- Commented overlay -->
    <div v-if="isCommented" class="absolute inset-0 z-20 bg-gray-100 opacity-25" />
    <!-- Monaco-like line numbers on the left margin -->
    <span
      v-if="editor.showLineNumbers"
      class="duration-50 absolute top-[3px] w-6 select-none text-right not-italic transition-colors"
      :style="{ left: -30 + 'px' }"
      :class="{
        'text-sm': editor.textSmall,
        'text-md': !editor.textSmall,
        'font-mono': editor.fontMono,
        'text-orange-200': !isFocused && !isCommentish,
        'text-gray-200': !isFocused && isCommentish,
        'font-bold text-orange-600': isFocused && !isCommentish,
        'font-bold text-gray-400': isFocused && isCommentish,
      }"
    >
      {{ lineNumberBase + 1 }}
    </span>
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
      class="duration-50 absolute -left-0.5 top-0 h-full w-1.5 transition-colors"
      :class="{
        'group-hover/statement:bg-orange-50': !isFocused,
        'bg-orange-100': isFocused && !isEditing && !isCommentish,
        'bg-gray-100': isFocused && !isEditing && isCommentish,
      }"
    />
    <!-- Statement focus indicator (all around if editing) -->
    <template v-if="isEditing">
      <div
        class="duration-50 absolute top-0 left-0 h-0.5 w-full transition-colors"
        :class="isCommentish ? 'bg-gray-100' : 'bg-orange-100'"
      />
      <div
        class="duration-50 absolute bottom-0 left-0 h-0.5 w-full transition-colors"
        :class="isCommentish ? 'bg-gray-100' : 'bg-orange-100'"
      />
      <div
        class="duration-50 absolute top-0 left-0 h-full w-0.5 transition-colors"
        :class="isCommentish ? 'bg-gray-100' : 'bg-orange-100'"
      />
      <div
        class="duration-50 absolute right-0 top-0 h-full w-0.5 transition-colors"
        :class="isCommentish ? 'bg-gray-100' : 'bg-orange-100'"
      />
    </template>
    <!-- Main cell -->
    <div
      class="relative py-1 px-2"
      :class="{
        'text-sm': editor.textSmall,
        'text-md': !editor.textSmall,
      }"
    >
      <component ref="rootCellRef" :is="rootCell.component" v-bind="rootCell.props" />
    </div>
    <!-- Debug info -->
    <div v-if="editor.debug" class="absolute top-2 -right-1 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
      <template v-if="isFocused">f</template>
      <template v-if="containerFocused">*</template>
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

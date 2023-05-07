<script lang="ts" setup>
import CodeDefinitionCell from "@/components/cells/CodeDefinitionCell.vue";
import CommentCell from "@/components/cells/CommentCell.vue";
import DataDefinitionCell from "@/components/cells/DataDefinitionCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import ProtoCell from "@/components/cells/ProtoCell.vue";
import TaskDefinitionCell from "@/components/cells/TaskDefinitionCell.vue";
import TypeDefinitionCell from "@/components/cells/TypeDefinitionCell.vue";
import { useMagicActions, useNavigationContext } from "@/components/file";
import { STATEMENT_CONTEXT, type StatementContext } from "@/components/statement";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { useCurrentEvaluations } from "@/state/evaluations";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { isSymbolStale, localErrorsOf, symbolOf, useSymbolOps } from "@/state/runtime";
import { setDragData, useRelativeDropZone } from "@/utils/drop";
import { METRIC_METER_UNITS, toBars, toFixed, toPercent, type MetricSet } from "@/utils/metrics";
import { DocumentDuplicateIcon, PlayIcon, PlusIcon, SparklesIcon, XCircleIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, useFocus, useFocusWithin, useKeyModifier, whenever } from "@vueuse/core";
import {
  computed,
  nextTick,
  provide,
  ref,
  watch,
  onBeforeUnmount,
  type Component,
  type ComputedRef,
  type Ref,
} from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  depth: number;
  ancestors: FragmentType<typeof StatementContentType>[];
  readonly: boolean;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  lineNumberBase: number;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const ancestors = computed(() => props.ancestors.map((s) => useFragment(StatementContentType, s)));

const editor = useEditorState();
const nav = useNavigationContext();

const isFocused = computed(() => editor.focusedElementId == statement.value?.id);
const isEditing = computed(() => isFocused.value && editor.editingElement);
const isSelected = computed(() => editor.isSelected(statement.value));
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented || ancestors.value.find((s) => s.commented));
const isCommentish = computed(
  () => isComment.value || isCommented.value || statement.value.type == StatementType.Blank
);
const lineNumber = computed(() => nav.value?.statementPositions[statement.value.id] + 1 ?? 0);
const lineNumberDigits = computed(() => lineNumber.value.toString().length);

// ancestor is considered highlighted if it's focused or selected (need to expand highlight to their depth)
const ancestorHighlightDepth = computed(() =>
  ancestors.value.findIndex((s) => editor.focusedElementId == s.id || editor.selectedElementIds.includes(s.id))
);
const isAncestorHighlight = computed(() => !editor.editingElement && ancestorHighlightDepth.value > -1);
const contentOffsetX = computed(() => props.depth * 20);
const highlightOffsetX = computed(() =>
  isAncestorHighlight.value ? ancestorHighlightDepth.value * 20 : contentOffsetX.value
);

// provide context
const destroyed = ref(false); // (useful for delete tracking if component had no time to update)
onBeforeUnmount(() => {
  destroyed.value = true;
});
const context: Ref<StatementContext> = computed(() => ({
  readonly: editor.readonly || props.readonly,
  focused: isFocused.value,
  editing: isEditing.value,
  depth: props.depth,
  xOffset: contentOffsetX.value,
  lineNumberBase: props.lineNumberBase,
  statement: props.statement,
  reference: symbolOf(statement.value.reference?.id) ?? null,
  file: props.file,
  destroyed: destroyed.value,
}));
provide(STATEMENT_CONTEXT, context);
const actions = useActions();
const magic = useMagicActions(statement as Ref<StatementHeader | null>);

// manage cells
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

const wrapperRef = ref<HTMLElement | null>(null);
const containerRef = ref<HTMLElement | null>(null);
const rootCellRef = ref<InstanceType<typeof ProtoCell>>();
const { focused: inContainerFocused } = useFocusWithin(containerRef);
const { focused: containerFocused } = useFocus(containerRef);
const { focused: inRootCellFocused } = useFocusWithin(rootCellRef);

// focus containerRef if focused in editor but not in container and not editing
watch(
  () => [isFocused.value, isEditing.value, containerFocused.value],
  () => {
    if (isFocused.value && !isEditing.value && !inRootCellFocused.value && !containerFocused.value) {
      containerFocused.value = true;
    }
  }
);

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
whenever(
  isEditing,
  () => {
    if (isEditing.value && (!inContainerFocused.value || containerFocused.value)) {
      rootCellRef.value?.focus();
      nextTick(() => rootCellRef.value?.focus()); // required to focus if just loaded
    }
  },
  { immediate: true }
);

// blur root cell if focused in container (but no longer editing or focused)
watch(
  () => [isEditing.value, inRootCellFocused.value],
  () => {
    if (!isEditing.value && inRootCellFocused.value) {
      rootCellRef.value?.blur();
    }
  }
);

const altKeyState = useKeyModifier("Alt");
const shiftKeyState = useKeyModifier("Shift");
// cancel focus if clicked outside (unless alt/shift is pressed)
onClickOutside(containerRef, () => {
  if (isFocused.value && !altKeyState.value && !shiftKeyState.value) {
    // We don't blur the root cell here because the focus is already elsewhere.
    editor.blurElement(statement.value as StatementHeader);
  }
});

// if anything inside the container becomes focused (except the container), enable editing mode
// (unless alt is pressed) :AltKeyEditing
whenever(inRootCellFocused, () => {
  if (!isFocused.value) {
    focusInEditor();
  }
  if (altKeyState.value || containerFocused.value) {
    return;
  }
  if (!isEditing.value && !editor.readonly) {
    editor.editElement(statement.value as StatementHeader);
  }
});

function focusInEditor() {
  editor.focusFile(file.value as any);
  editor.focusElement(statement.value as StatementHeader);
}

function onClickContainer(e: MouseEvent) {
  // ignore if alt was pressed :AltKeyEditing
  if (e.altKey) {
    return;
  }
  // create selection to here if shift was pressed
  if (e.shiftKey) {
    const index = nav.value.statementPositions[statement.value.id];
    const lastIndex = nav.value.statementPositions[editor.focusedElementId ?? ""];
    console.log("select all statements between", index, lastIndex);
    focusInEditor();
    if (lastIndex != null) {
      // select all statements between
      for (let i = Math.min(index, lastIndex); i <= Math.max(index, lastIndex); i++) {
        const statement = nav.value.statements[i];
        if (statement != null) {
          editor.addToSelection(statement);
        }
      }
    }
    return;
  }
  if (!isFocused.value) {
    focusInEditor();
  }
  if (!editor.readonly) {
    editor.editElement(statement.value as StatementHeader);
    containerFocused.value = false;
  } else {
    containerFocused.value = true;
  }
  if (!inContainerFocused.value) {
    rootCellRef.value?.focus();
  }
}

function insertStatementBelow(e: MouseEvent) {
  onClickContainer(e);
  magic.insertBelow(true);
  e.preventDefault();
  e.stopPropagation();
}

// drag & drop
const innerDrag = computed(() => (rootCellRef.value as any)?.innerDrag == true);
const {
  isOverDropZone: dragOver,
  inTopHalf: dragInTopHalf,
  inBottomHalf: dragInBottomHalf,
} = useRelativeDropZone(
  containerRef,
  ["Statement", "File"],
  onDrop,
  computed(() => !innerDrag.value)
);

function onDragStart(e: DragEvent) {
  if (containerRef.value == null) return;
  if (e.dataTransfer == null) throw new Error("no dataTransfer??");
  setDragData(e, { type: "Statement", id: statement.value.id });
  e.dataTransfer.setDragImage(containerRef.value, 0, 0);
}

async function onDrop(thing: File[] | { type: string; id: string } | null) {
  if (thing == null) return;
  if (Array.isArray(thing)) {
    console.log("drop insert files into new statement", thing);
    await magic.insertFilesAsDataset(dragInTopHalf.value ? "above" : "below", thing);
  } else if (thing?.type == "Statement") {
    const targetStatement = nav.value.statementsById[thing.id];
    if (thing.id == statement.value.id || targetStatement == null) {
      return;
    }
    const dropLocation = dragInTopHalf.value
      ? nav.value.getLocationRightAbove(statement.value as StatementHeader)
      : nav.value.getLocationRightBelow(statement.value as StatementHeader);
    console.log("drop move statement", thing, dropLocation);
    await nav.value.moveTo(targetStatement as StatementHeader, dropLocation);
  } else {
    throw new Error("unexpected drop");
  }
}

// runtime
const localErrors = localErrorsOf(statement);
const hasLocalErrors = computed(() => (localErrors.value?.length ?? 0) > 0);
const isStale = isSymbolStale(statement);
const evaluations = useCurrentEvaluations();

const metricSets: ComputedRef<MetricSet[] | null> = computed(() => {
  if (statement.value.type != StatementType.Definition || statement.value.symbolType == SymbolType.Build) {
    return null;
  }
  const globalMetrics = evaluations.getGlobalEvaluation(statement.value.id)?.aggregatedMetrics;
  const metricSets = [];
  if (globalMetrics == null) {
    return null;
  }

  // global
  metricSets.push({
    label: "Global",
    metrics: [
      {
        label: "Clarity",
        value: toPercent(globalMetrics?.clarity),
        bars: toBars(globalMetrics?.clarity, "clarity"),
      },
      {
        label: "Difficulty",
        value: toFixed(globalMetrics?.difficulty),
        bars: toBars(globalMetrics?.difficulty, "difficulty"),
      },
    ],
  });

  for (const buildEval of evaluations.getBuildEvaluations(statement.value.id)) {
    const buildMetrics = buildEval?.aggregatedMetrics;
    const localMetrics = [
      {
        label: "Performance",
        value: buildMetrics?.performance,
        bars: toBars(buildMetrics?.performance, "performance"),
      },
      {
        label: "Speed",
        value: buildMetrics?.speed,
        bars: toBars(buildMetrics?.speed, "speed"),
        unavailable: statement.value.symbolType != SymbolType.Task,
      },
    ];
    metricSets.push({
      label: "claude",
      metrics: localMetrics,
    });
  }

  return metricSets;
});

// symbol ops (inline)
type InlineAction = {
  label: string;
  icon: any;
  action: () => void;
};
const symbolOps = useSymbolOps();
const inlineActions = computed(() => {
  if (statement.value.type != StatementType.Definition) {
    return [];
  }
  const inlineActions: InlineAction[] = [];
  if (!context.value.readonly) {
    inlineActions.push({
      label: "Duplicate",
      icon: DocumentDuplicateIcon,
      action: () => magic.duplicate(),
    });
  }
  // :BuildEvaluate disabled for now
  // if (statement.value.symbolType == SymbolType.Build || statement.value.symbolType == SymbolType.Task) {
  //   inlineActions.push({
  //     label: "Build",
  //     icon: WrenchIcon,
  //     action: () => symbolOps.build(statement.value, BuildScope.Reactive),
  //   });
  // }
  if (statement.value.symbolType == SymbolType.Task || statement.value.symbolType == SymbolType.Code) {
    inlineActions.push({
      label: "Run",
      icon: PlayIcon,
      action: () => symbolOps.openRun(statement.value),
    });
  }
  // if (
  //   statement.value.symbolType == SymbolType.Task ||
  //   statement.value.symbolType == SymbolType.Expectation ||
  //   statement.value.symbolType == SymbolType.Build
  // ) {
  //   inlineActions.push({
  //     label: "Evaluate",
  //     icon: CheckCircleIcon,
  //     action: () => symbolOps.evaluate(statement.value),
  //   });
  // }
  return inlineActions;
});
</script>
<template>
  <!-- Statement wrapper -->
  <div
    class="group/statement relative w-full px-[50px]"
    ref="wrapperRef"
    @click="onClickContainer"
    @dragstart="onDragStart"
  >
    <!-- Statement main -->
    <div
      tabindex="-1"
      ref="containerRef"
      class="relative min-h-[30px] w-full outline-none transition duration-75 focus:outline-none"
      :class="{
        'focus:bg-orange-100': !isCommentish,
        'focus:bg-gray-100': isCommentish,
        'bg-orange-100': !isCommentish && (isSelected || isAncestorHighlight || dragOver),
        'bg-gray-100': isCommentish && (isSelected || isAncestorHighlight || dragOver),
        'font-mono': editor.fontMono && !isComment,
        'text-gray-700': isCommented,
      }"
      :style="{
        marginLeft: highlightOffsetX + 'px',
        paddingLeft: contentOffsetX - highlightOffsetX + 'px',
        width: `calc(100% - ${highlightOffsetX}px)`,
      }"
    >
      <!-- Add statement below button -->
      <!-- z-[5] to put it over the line numbers, which have a fixed width to make positioning easier (don't expect >99 statements/file) -->
      <button
        v-if="!context.readonly"
        class="invisible absolute top-[3px] z-[5] rounded-sm p-0.5 text-gray-500 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:visible"
        :style="{
          transform: 'translateX(' + (-30 - lineNumberDigits * 8 + 'px') + ')',
        }"
        @click="insertStatementBelow"
      >
        <PlusIcon class="h-4 w-4" />
      </button>
      <!-- Monaco-like line numbers on the left margin -->
      <span
        class="invisible absolute top-[3px] w-6 cursor-grab select-none text-right not-italic transition duration-75"
        :style="{ transform: 'translateX(' + -30 + 'px)' }"
        :class="{
          ' group-focus-within/statement:visible group-hover/statement:visible': !editor.showLineNumbers,
          'text-sm': editor.textSmall,
          'text-md': !editor.textSmall,
          'font-mono': editor.fontMono,
          'text-orange-200 group-focus-within/statement:font-bold group-focus-within/statement:text-orange-500 group-hover/statement:font-bold group-hover/statement:text-orange-500 group-focus/statement:text-orange-500':
            !isCommentish,
          'text-gray-200 group-focus-within/statement:font-bold group-focus-within/statement:text-gray-500 group-hover/statement:font-bold group-hover/statement:text-gray-500 group-focus/statement:text-gray-500':
            isCommentish,
          'text-orange-500': dragOver && !isCommentish,
          'text-gray-500': dragOver && isCommentish,
        }"
        @mousedown="wrapperRef?.setAttribute('draggable', 'true')"
        @mouseup="wrapperRef?.setAttribute('draggable', 'false')"
      >
        {{ lineNumberBase + 1 }}
      </span>
      <!-- Left gutter indicators (beneath line numbers) -->
      <div
        v-if="statement.generated"
        class="absolute top-[28px] select-none"
        :style="{ transform: 'translateX(' + -19 + 'px)' }"
      >
        <SparklesIcon
          class="h-4 w-4"
          :class="{
            'text-gray-200 group-focus-within/statement:text-gray-500 group-hover/statement:text-gray-500': isStale,
            'text-orange-200 group-focus-within/statement:text-orange-500 group-hover/statement:text-orange-500':
              !isStale,
          }"
        />
      </div>
      <!-- TODO @UX: focus on @mousedown would be more responsive but doesn't focus properly.. -->
      <!-- Commented overlay (TODO @UX: commented overlay is ugly) -->
      <div v-if="isCommented" class="absolute inset-0 z-[8] bg-gray-100 opacity-25" />
      <!-- Statement focus indicator (left side if not editing) -->
      <!-- (the z-[5] puts it in front of the statement focus border) -->
      <div
        class="absolute -left-0.5 top-0 z-[5] h-full w-1.5 transition-colors duration-75"
        :class="{
          'group-focus-within/statement:bg-orange-200 group-hover/statement:bg-orange-300': !isCommentish,
          'group-focus-within/statement:bg-gray-200 group-hover/statement:bg-gray-300': isCommentish,
        }"
      />
      <!-- Statement focus indicator (all around if editing) -->
      <template v-if="isEditing">
        <div
          class="duration-50 absolute left-0 top-0 h-0.5 w-full transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
        <div
          class="duration-50 absolute bottom-0 left-0 h-0.5 w-full transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
        <div
          class="duration-50 absolute left-0 top-0 h-full w-0.5 transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
        <div
          class="duration-50 absolute right-0 top-0 h-full w-0.5 transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
      </template>
      <!-- Statement drag & drop indicator (top/bottom) -->
      <div
        v-if="!readonly && dragOver && dragInTopHalf"
        class="duration-50 absolute -top-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition-colors"
      />
      <div
        v-if="!readonly && dragOver && dragInBottomHalf"
        class="duration-50 absolute -bottom-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition-colors"
      />
      <!-- Main cell -->
      <div
        class="relative px-2 py-1"
        :class="{
          'text-sm': editor.textSmall,
          'text-md': !editor.textSmall,
        }"
      >
        <!-- Most cells handle these events themselves, this is for raw DeclarationCells -->
        <component
          v-if="rootCell.component == DeclarationCell"
          ref="rootCellRef"
          :is="rootCell.component"
          @navigate-up="magic.moveFocusUp"
          @navigate-down="magic.moveFocusDown"
        />
        <component v-else ref="rootCellRef" :is="rootCell.component" v-bind="rootCell.props" />
        <!-- Inline cell actions -->
        <span
          v-if="inlineActions.length > 0"
          :class="[isFocused ? '' : 'invisible']"
          class="absolute right-0 top-0 flex flex-row items-center gap-1 p-1 group-hover/statement:visible"
        >
          <button
            v-for="action in inlineActions"
            :key="action.label"
            class="p-0.5 text-gray-500 hover:bg-orange-100 hover:text-gray-800"
            @click.prevent.stop="action.action"
          >
            <component :is="action.icon" class="h-4 w-4" />
          </button>
        </span>
      </div>
      <!-- Gutter indicators on the right margin -->
      <div
        v-if="statement.type == StatementType.Definition || statement.type == StatementType.Reference"
        class="absolute left-full top-[6px] flex origin-top-right select-none flex-row gap-2 px-1 not-italic"
        :class="{
          'text-md': !editor.textSmall,
          'text-sm': editor.textSmall,
        }"
      >
        <!-- Metrics -->
        <div v-if="editor.inlineMetrics && metricSets != null" class="flex flex-row gap-1.5">
          <div v-for="metricSet of metricSets" :key="metricSet.label" class="flex flex-row gap-0.5">
            <span v-for="metric in metricSet.metrics" :key="metric.label">
              <svg viewBox="0 0 6 24" class="h-5">
                <rect
                  v-for="i in METRIC_METER_UNITS"
                  :key="i"
                  x="0"
                  :y="(i - 1) * 6"
                  width="6"
                  height="4"
                  :fill="
                    metric.unavailable ?? false
                      ? 'transparent'
                      : i > METRIC_METER_UNITS - metric.bars
                      ? 'skyblue'
                      : 'lightgrey'
                  "
                />
              </svg>
            </span>
          </div>
        </div>
        <!-- Errors/warnings -->
        <div>
          <!-- Errors -->
          <button
            v-if="hasLocalErrors"
            class="flex rounded-sm font-bold text-red-700 underline-offset-4 hover:bg-red-100 hover:text-red-900"
            @click="actions.apply('editor.view.openIssues')"
          >
            <XCircleIcon class="h-5 w-5" />
          </button>
          <!-- Warnings -->
        </div>
        <!-- don't exist yet -->
      </div>
    </div>
    <!-- Debug info -->
    <div v-if="editor.debug" class="absolute -right-1 top-2 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
      <template v-if="isAncestorHighlight">h{{ ancestorHighlightDepth }}</template>
      <template v-if="isFocused">F</template>
      <template v-if="isSelected">S</template>
      <template v-if="inContainerFocused">*</template>
      <template v-if="containerFocused">.</template>
      <template v-if="isEditing">e</template>
      <template v-if="isFirstInGroup">[</template>
      <template v-if="isLastInGroup">]</template>
      <template v-if="isCommented">#</template>
      <template v-if="isStale">S</template>
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

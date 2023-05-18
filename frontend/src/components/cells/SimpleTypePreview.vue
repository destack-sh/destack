<script lang="ts" setup>
import { TypeHint, TypeTag, type SimpleType } from "@/gql/graphql";
import { renderBuiltinType, TYPETAG_KEYWORD } from "@/state/editor";
import { symbolOf, TypeFlag } from "@/state/runtime";
import {
  AdjustmentsHorizontalIcon,
  ArrowDownCircleIcon,
  ArrowUpRightIcon,
  AtSymbolIcon,
  Bars3BottomLeftIcon,
  CalendarDaysIcon,
  CheckIcon,
  ChevronDoubleDownIcon,
  ClockIcon,
  CodeBracketIcon,
  DocumentIcon,
  FingerPrintIcon,
  HashtagIcon,
  LinkIcon,
  LockClosedIcon,
  MinusSmallIcon,
  PhoneIcon,
  PhotoIcon,
  SparklesIcon,
  SpeakerWaveIcon,
  Squares2X2Icon,
  VideoCameraIcon,
} from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{
  type: SimpleType;
  showTypeName?: boolean;
  hideIcon?: boolean;
  hideFlags?: boolean;
  hideReference?: boolean;
}>();

// resolve type since the type reference is likely not included
// (this hack will be removed once the special interp state finally dies)
const resolvedReference = computed(() => {
  if (props.type.reference == null) {
    return;
  } else if (props.type.reference.name != null) {
    return props.type.reference;
  } else {
    return symbolOf(props.type.reference.id);
  }
});

const resolvedTag = computed(() => resolvedReference.value?.rootTypeTag ?? props.type.tag);

const iconsByTag: Record<TypeTag, any> = {
  [TypeTag.String]: Bars3BottomLeftIcon,
  [TypeTag.Number]: HashtagIcon,
  [TypeTag.Boolean]: CheckIcon,
  [TypeTag.Embedding]: SparklesIcon,
  [TypeTag.Null]: MinusSmallIcon,
  [TypeTag.File]: DocumentIcon,
  [TypeTag.Image]: PhotoIcon,
  [TypeTag.Audio]: SpeakerWaveIcon,
  [TypeTag.Video]: VideoCameraIcon,
  [TypeTag.TypeReference]: ArrowUpRightIcon,
  [TypeTag.Struct]: Squares2X2Icon,
  [TypeTag.Enum]: ChevronDoubleDownIcon,
};
const iconsByHint: Record<TypeHint, any> = {
  // string
  [TypeHint.Uuid]: FingerPrintIcon,
  [TypeHint.Date]: CalendarDaysIcon,
  [TypeHint.Datetime]: CalendarDaysIcon,
  [TypeHint.Time]: ClockIcon,
  [TypeHint.Duration]: ClockIcon,
  [TypeHint.Url]: LinkIcon,
  [TypeHint.EmbedUrl]: LinkIcon,
  [TypeHint.Email]: AtSymbolIcon,
  [TypeHint.Markdown]: CodeBracketIcon,
  [TypeHint.Html]: CodeBracketIcon,
  [TypeHint.Code]: CodeBracketIcon,
  // number
  [TypeHint.Integer]: HashtagIcon,
  [TypeHint.Float]: HashtagIcon,
  [TypeHint.Slider]: AdjustmentsHorizontalIcon,
  [TypeHint.Phone]: PhoneIcon,
  // boolean
  [TypeHint.Toggle]: CheckIcon,
  [TypeHint.Checkbox]: CheckIcon,
};

const icon = computed(() => {
  if (props.type.hint != null && iconsByHint[props.type.hint] != null) {
    return iconsByHint[props.type.hint];
  } else {
    return iconsByTag[resolvedTag.value];
  }
});
</script>
<template>
  <div class="relative inline-flex flex-row items-baseline gap-2">
    <!-- Force icon to align with text -->
    <!-- works fine but there has to be a better way... -->
    <span v-if="icon && !hideIcon" class="h-4 w-4">
      <span class="opacity-0">t</span>
      <component :is="icon" class="absolute left-0 h-4 w-4" :class="showTypeName ? 'top-0.5' : 'top-0'" />
    </span>
    <span v-if="!icon || (showTypeName && type.reference == null)">{{
      renderBuiltinType(resolvedTag, type.hint ?? null)
    }}</span>
    <span v-if="type.reference && !hideReference">{{ resolvedReference?.name ?? "???" }}</span>
    <span v-else-if="type.tag == TypeTag.TypeReference && type.reference == null">...</span>
    <LockClosedIcon v-if="type.flags & TypeFlag.IsSecret && !hideFlags" class="-ml-1 h-4 w-4" />
  </div>
</template>

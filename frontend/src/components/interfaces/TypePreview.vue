<script lang="ts" setup>
import { TypeHint, TypeTag, type Field, StatementType } from "@/gql/graphql";
import { TypeFlag, useCurrentModule, useNavigation } from "@/state/module";
import { getStatementIconOutline } from "@/state/statement";
import { renderBuiltinType } from "@/state/type";
import {
  AdjustmentsHorizontalIcon,
  ArrowsRightLeftIcon,
  AtSymbolIcon,
  Bars3BottomLeftIcon,
  CalendarDaysIcon,
  CheckIcon,
  ClockIcon,
  CodeBracketIcon,
  DocumentIcon,
  FingerPrintIcon,
  HandThumbUpIcon,
  HashtagIcon,
  IdentificationIcon,
  KeyIcon,
  LinkIcon,
  ListBulletIcon,
  LockClosedIcon,
  MinusSmallIcon,
  PhoneIcon,
  PhotoIcon,
  SparklesIcon,
  SpeakerWaveIcon,
  StarIcon,
  VideoCameraIcon,
} from "@heroicons/vue/24/outline";
import { useKeyModifier } from "@vueuse/core";
import { computed } from "vue";

const props = defineProps<{
  type: Field;
  showTypeName?: boolean;
  hideIcon?: boolean;
  hideFlags?: boolean;
  hideReference?: boolean;
}>();

const module = useCurrentModule();
const resolvedReference = computed(() => {
  if (props.type.reference == null) {
    return;
  } else if (props.type.reference.name != null) {
    return props.type.reference;
  } else {
    return module.statementOf(props.type.reference.id);
  }
});
const nav = useNavigation();
const altState = useKeyModifier("Alt");

const resolvedTag = computed(() => resolvedReference.value?.rootTypeTag ?? props.type.tag);

const iconsByTag: Partial<Record<TypeTag, any>> = {
  [TypeTag.String]: Bars3BottomLeftIcon,
  [TypeTag.Number]: HashtagIcon,
  [TypeTag.Boolean]: CheckIcon,
  [TypeTag.Vector]: SparklesIcon,
  [TypeTag.Null]: MinusSmallIcon,
  [TypeTag.File]: DocumentIcon,
  [TypeTag.Struct]: getStatementIconOutline(StatementType.Type, TypeTag.Struct),
  [TypeTag.Enum]: getStatementIconOutline(StatementType.Type, TypeTag.Enum),
};
const iconsByHint: Partial<Record<TypeHint, any>> = {
  // string
  [TypeHint.Name]: IdentificationIcon,
  [TypeHint.Uuid]: FingerPrintIcon,
  [TypeHint.Date]: CalendarDaysIcon,
  [TypeHint.Datetime]: CalendarDaysIcon,
  [TypeHint.Time]: ClockIcon,
  [TypeHint.Duration]: ClockIcon,
  [TypeHint.Url]: LinkIcon,
  [TypeHint.Email]: AtSymbolIcon,
  [TypeHint.Markdown]: CodeBracketIcon,
  [TypeHint.Html]: CodeBracketIcon,
  [TypeHint.Code]: CodeBracketIcon,
  [TypeHint.Key]: KeyIcon,
  [TypeHint.Secret]: LockClosedIcon,
  // number
  [TypeHint.Integer]: HashtagIcon, // should have a different icon from float
  [TypeHint.Float]: HashtagIcon,
  [TypeHint.Slider]: AdjustmentsHorizontalIcon,
  [TypeHint.Phone]: PhoneIcon,
  [TypeHint.Rating]: StarIcon,
  // boolean
  [TypeHint.Toggle]: ArrowsRightLeftIcon,
  [TypeHint.Checkbox]: CheckIcon,
  [TypeHint.Thumbs]: HandThumbUpIcon,
  // file
  [TypeHint.Audio]: SpeakerWaveIcon,
  [TypeHint.Video]: VideoCameraIcon,
  [TypeHint.Image]: PhotoIcon,
};

const icon = computed(() => {
  if (
    resolvedReference.value != null &&
    [StatementType.Code, StatementType.Flow, StatementType.Task, StatementType.Dataset].includes(
      resolvedReference.value?.type
    )
  ) {
    return getStatementIconOutline(resolvedReference.value?.type);
  } else if (props.type.hint != null && iconsByHint[props.type.hint] != null) {
    return iconsByHint[props.type.hint];
  } else if (iconsByTag[resolvedTag.value] != null) {
    return iconsByTag[resolvedTag.value];
  } else {
    return null;
  }
});
</script>
<template>
  <div class="relative whitespace-nowrap">
    <!-- Force icon to align with text -->
    <!-- works fine but there has to be a better way... -->
    <div v-if="icon && !hideIcon" class="inline-block h-4 w-6">
      &nbsp;
      <component :is="icon" class="absolute left-0 top-0.5 h-4 w-4" :class="showTypeName ? 'top-0.5' : 'top-0'" />
    </div>
    <span v-if="!icon || (showTypeName && type.reference == null)">
      {{ renderBuiltinType(resolvedTag, type.hint ?? null) }}
    </span>
    <span
      v-if="(resolvedTag == TypeTag.TypeReference || type.reference) && !hideReference"
      class="mx-1"
      :class="[altState ? 'decoration-gray-500 underline-offset-4 hover:underline' : '']"
      @click="
        (e) => {
          if (altState && resolvedReference != null) {
            e.stopPropagation();
            e.preventDefault();
            nav.focusStatement(resolvedReference);
          }
        }
      "
    >
      {{ resolvedReference?.name ?? "???" }}
    </span>
    <!-- TODO @UX: show required type flag -->
    <!-- Flags -->
    <div v-if="type.flags & TypeFlag.IsArray && !hideFlags" class="relative left-1 mr-1 inline-block h-4 w-4">
      <ListBulletIcon class="absolute left-0 top-0.5 h-4 w-4" />
    </div>
  </div>
</template>

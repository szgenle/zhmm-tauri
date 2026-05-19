<script setup lang="ts">
import { computed } from "vue";
import { assessStrength, StrengthLevel } from "../utils/passwordStrength";

const props = defineProps<{ password: string }>();

const result = computed(() => assessStrength(props.password));

const colorMap: Record<StrengthLevel, string> = {
  [StrengthLevel.VERY_WEAK]: "#d03050",
  [StrengthLevel.WEAK]: "#f0a020",
  [StrengthLevel.FAIR]: "#e8c010",
  [StrengthLevel.STRONG]: "#18a058",
  [StrengthLevel.VERY_STRONG]: "#2080f0",
};

const barColor = computed(() => colorMap[result.value.level]);
</script>

<template>
  <div v-if="password" class="strength-bar">
    <div class="strength-row">
      <n-progress
        type="line"
        :percentage="result.score"
        :color="barColor"
        :show-indicator="false"
        :height="6"
        style="flex: 1"
      />
      <n-text :style="{ color: barColor, fontSize: '12px', marginLeft: '8px', whiteSpace: 'nowrap' }">
        {{ result.label }}
      </n-text>
    </div>
    <n-text v-if="result.hint" depth="3" style="font-size: 11px; margin-top: 2px; display: block">
      {{ result.hint }}
    </n-text>
  </div>
</template>

<style scoped>
.strength-bar {
  margin-top: 4px;
}
.strength-row {
  display: flex;
  align-items: center;
}
</style>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ref, watch, onMounted, onUnmounted } from 'vue';
import { configManager } from '../utils/ConfigManager';

const props = defineProps<{
  currentView: string;
}>();

const emit = defineEmits<{
  (e: 'update:currentView', view: string): void;
}>();

// Navigation Items Configuration
const navLabels: Record<string, string> = {
    filter: '过滤器本地修改',
    market: '市集',
    workshop: '白菜工坊',
    poedb: 'POE2DB'
};
const navItems = ref<string[]>([]);
const navContainerRef = ref<HTMLElement | null>(null);

// Drag State
const isDragging = ref(false);
const draggingItem = ref<string | null>(null);
const ghostPos = ref({ x: 0, y: 0 });
let dragStartTimer: number | null = null;
const dragStartDelay = 220; // ms long-press threshold to start reordering

// Initialize Order
const initNavOrder = async () => {
    // Ensure config is ready
    await configManager.init();
    const settings = configManager.getSettings();
    if (settings.navOrder && settings.navOrder.length > 0) {
        // Filter out any IDs that might not exist in our label map anymore
        navItems.value = settings.navOrder.filter(id => navLabels[id]);
        
        // Add any missing new items to the end
        const currentKeys = Object.keys(navLabels);
        for (const key of currentKeys) {
            if (!navItems.value.includes(key)) {
                navItems.value.push(key);
            }
        }
    } else {
        // Default order
         navItems.value = ['filter', 'market', 'workshop', 'poedb'];
    }
};

onMounted(() => {
    initNavOrder();
    window.addEventListener('mousemove', handleGlobalMouseMove);
    window.addEventListener('mouseup', handleGlobalMouseUp);
});

onUnmounted(() => {
    window.removeEventListener('mousemove', handleGlobalMouseMove);
    window.removeEventListener('mouseup', handleGlobalMouseUp);
});


// Drag Handling
const cancelDragTimer = () => {
  if (dragStartTimer !== null) {
    clearTimeout(dragStartTimer);
    dragStartTimer = null;
  }
};

const handleMouseDown = (e: MouseEvent, item: string) => {
  // Only left click
  if (e.button !== 0) return;

  // Track current mouse position for the ghost even before drag starts
  ghostPos.value = { x: e.clientX, y: e.clientY };

  cancelDragTimer();
  dragStartTimer = window.setTimeout(() => {
    isDragging.value = true;
    draggingItem.value = item;
  }, dragStartDelay);

  // Prevent text selection on long press
  e.preventDefault();
};

const handleGlobalMouseMove = (e: MouseEvent) => {
  // Keep ghost position updated so the ghost appears where the cursor is when drag starts
  ghostPos.value = { x: e.clientX, y: e.clientY };

  if (!isDragging.value || !draggingItem.value || !navContainerRef.value) return;

    ghostPos.value = { x: e.clientX, y: e.clientY };
    
    // Hit testing
    // We get all tab elements in the container
    const children = Array.from(navContainerRef.value.children) as HTMLElement[];
    const draggedIndex = navItems.value.indexOf(draggingItem.value);
    
    // Find which item we are hovering over
    // Simple heuristic: Mouse X is within the bounding box of a tab
    // We map mouse X to an index
    
    let targetIndex = -1;
    
    for (let i = 0; i < children.length; i++) {
        const el = children[i];
        // Skip the ghost if it's in the DOM (it won't be in navItems, but check anyway)
        const rect = el.getBoundingClientRect();
        
        // Logic: specific "Swap" threshold
        // If we move mouse past the center of the neighbor, swap.
        if (e.clientX >= rect.left && e.clientX <= rect.right) {
            targetIndex = i;
            break;
        }
    }
    
    if (targetIndex !== -1 && targetIndex !== draggedIndex) {
        // Perform Swap
        const newItems = [...navItems.value];
        const item = newItems[draggedIndex];
        newItems.splice(draggedIndex, 1);
        newItems.splice(targetIndex, 0, item);
        navItems.value = newItems;
        // Don't save yet, wait for mouse up
    }
};

const handleGlobalMouseUp = async () => {
  cancelDragTimer();

  if (isDragging.value) {
    isDragging.value = false;
    draggingItem.value = null;
        
    // Persist
    await configManager.saveSettings({ navOrder: navItems.value });
  } else {
    draggingItem.value = null;
  }
};

// --- Existing Logic ---
// track last non-settings view so we can toggle back
const lastNonSettings = ref<string>(props.currentView as string);
watch(
  () => props.currentView,
  (val) => {
    if (val !== 'settings') lastNonSettings.value = val;
  }
);

const appWindow = getCurrentWindow();

const minimize = () => appWindow.minimize();
const maximize = async () => {
  if (await appWindow.isMaximized()) {
    appWindow.unmaximize();
  } else {
    appWindow.maximize();
  }
};
const closeWindow = async () => {
  console.log('Close button clicked');
  try {
    await appWindow.close();
  } catch (error) {
    console.error('Failed to close window:', error);
  }
};

const onSettingsClick = () => {
  if (props.currentView === 'settings') {
    emit('update:currentView', lastNonSettings.value || 'filter');
  } else {
    emit('update:currentView', 'settings');
  }
};
</script>

<template>
  <div class="title-bar-container">
    <div class="nav-section" ref="navContainerRef">
      <div 
        v-for="item in navItems" 
        :key="item"
        class="nav-tab" 
        :class="{ active: currentView === item, 'is-dragging': draggingItem === item }" 
        @mousedown="handleMouseDown($event, item)"
        @click="!isDragging && emit('update:currentView', item)"
      >
        <span>{{ navLabels[item] }}</span>
        <div class="tab-indicator"></div>
      </div>
    </div>
    
    <!-- Ghost Element for visual feedback (follows mouse) -->
    <div v-if="isDragging && draggingItem" class="ghost-tab" :style="{ left: ghostPos.x + 'px', top: ghostPos.y + 'px' }">
        {{ navLabels[draggingItem] }}
    </div>

    <div class="controls-section">
      <div class="control-button settings-button" :class="{ active: currentView === 'settings' }" @click="onSettingsClick" title="Settings">
        <span class="gear-icon" aria-hidden="true"></span>
      </div>
      <div class="window-buttons-group">
        <div class="control-button win-btn" @click.stop.prevent="minimize" title="Minimize">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
            <path d="M1.2 5h7.6" />
          </svg>
        </div>
        <div class="control-button win-btn" @click.stop.prevent="maximize" title="Maximize">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
            <rect x="1.5" y="1.5" width="7" height="7" rx="1" />
          </svg>
        </div>
        <div class="control-button win-btn close-btn" @click.stop.prevent="closeWindow" title="Close">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 2l6 6M8 2l-6 6" />
          </svg>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.title-bar-container {
  height: 34px;
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(0, 0, 0, 0.2);
  backdrop-filter: blur(4px);
  user-select: none;
  -webkit-app-region: drag;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  box-sizing: border-box;
}

.nav-section {
  display: flex;
  height: 100%;
  padding-left: 8px;
  -webkit-app-region: no-drag;
}

.nav-tab {
  height: 100%;
  padding: 0 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  color: #cccccc;
  position: relative;
  cursor: pointer;
  transition: color 0.2s, background-color 0.2s;
}

.nav-tab:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.nav-tab.active {
  color: #ffffff;
  font-weight: bold;
}

.nav-tab.is-dragging {
    opacity: 0.3; /* Dim the original placeholder */
    background: rgba(255,255,255,0.05); /* Show slot clearly */
}

/* Ghost Tab Floating */
.ghost-tab {
    position: fixed;
    pointer-events: none;
    z-index: 9999;
    background: rgba(40, 44, 52, 0.9);
    border: 1px solid rgba(255,255,255,0.2);
    color: #fff;
    padding: 6px 14px;
    font-size: 14px;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    transform: translate(-50%, -50%); /* Center on mouse cursor */
    white-space: nowrap;
    backdrop-filter: blur(4px);
    font-weight: bold;
}

.tab-indicator {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 3px;
  background-color: #409eff;
  transform: scaleX(0);
  transition: transform 0.2s;
}

.nav-tab.active .tab-indicator {
  transform: scaleX(1);
}

.controls-section {
  display: flex;
  height: 100%;
  align-items: center;
  -webkit-app-region: no-drag;
  --btn-size: 40px;
  --settings-btn-size: 34px;
  --icon-color: #f5f5f7;
  --icon-opacity: 0.82;
  --hover-bg: rgba(255, 255, 255, 0.08);
  --pressed-bg: rgba(255, 255, 255, 0.16);
  /* crystal-style active settings background with base color for dark backgrounds */
  --settings-active-base: rgba(64,72,84,0.32);
  --settings-active-bg: linear-gradient(180deg, rgba(255,255,255,0.04), rgba(255,255,255,0.02));
  --settings-active-border: rgba(255,255,255,0.06);
  --settings-active-inset: rgba(255,255,255,0.06);
  --close-hover-bg: #f1707a;
  --close-pressed-bg: #d13438;
  --anim: 120ms ease;
}

.control-button {
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  transition: background-color var(--anim), color var(--anim), opacity var(--anim);
  -webkit-app-region: no-drag;
}

.settings-button {
  width: var(--settings-btn-size);
  height: 100%;
  margin-right: 6px;
  border-radius: 4px;
  transition: background-color var(--anim), opacity var(--anim);
}

.gear-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: var(--icon-color);
  opacity: var(--icon-opacity);
  transition: opacity var(--anim), color var(--anim);
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", "Segoe UI Symbol", sans-serif;
  font-size: 16px;
  line-height: 1;
}

.gear-icon::before {
  content: "\E713"; /* Fluent/MDL2 settings gear */
}

.settings-button:hover .gear-icon,
.settings-button.active .gear-icon {
  opacity: 1;
  color: #ffffff;
}

.settings-button:hover {
  background-color: var(--hover-bg);
}

.settings-button.active {
  background: var(--settings-active-bg), var(--settings-active-base);
  background-blend-mode: overlay;
  border: 1px solid var(--settings-active-border);
  box-shadow: inset 0 1px 0 var(--settings-active-inset), 0 6px 18px rgba(0,0,0,0.18);
  backdrop-filter: blur(6px);
}

.settings-button.active .gear-icon {
  color: #f8fbff;
  opacity: 1;
}

.window-buttons-group {
  height: 100%;
  display: flex;
  -webkit-app-region: no-drag;
}

.win-btn {
  width: var(--btn-size);
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--anim), color var(--anim), opacity var(--anim);
  -webkit-app-region: no-drag;
}

.win-btn svg {
  width: 10px;
  height: 10px;
  color: var(--icon-color);
  stroke: var(--icon-color);
  fill: none;
  opacity: var(--icon-opacity);
}

.win-btn:hover {
  background-color: var(--hover-bg);
}

.win-btn:hover svg {
  opacity: 1;
}

.close-btn:hover {
  background-color: var(--close-hover-bg) !important;
}

.close-btn:hover svg {
  color: #ffffff !important;
  stroke: #ffffff !important;
}

.win-btn:active {
  background-color: var(--pressed-bg);
}

.close-btn:active {
  background-color: var(--close-pressed-bg) !important;
}
</style>

<script setup lang="ts">
import { ref, onMounted, onActivated, nextTick, reactive, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import { configManager } from "../utils/ConfigManager";
import { FilterParser, type FilterBlock, type FilterLine } from '../utils/FilterParser';
import FilterRuleEditor from '../components/FilterRuleEditor.vue';

interface FilterFile {
  name: string;
  path: string;
}

// Global View State Persistence (Session-based)
// Maps filePath -> { expandedKey: string | null, scrollY: number }
const globalViewState = reactive<Record<string, { expandedKey: string | null, scrollY: number }>>({});

const filterFiles = ref<FilterFile[]>([]);
const selectedFile = ref<FilterFile | null>(null);
const parsedBlocks = ref<FilterBlock[]>([]);
const focusedBlockId = ref<string | null>(null);
const isLoading = ref(false);
const saveStatus = ref<string>("");
const currentViewMode = ref<'visual' | 'code'>('visual');
const rawContent = ref<string>("");
const visualListRef = ref<HTMLElement | null>(null);
const searchQuery = ref("");
// Buffer for copying attributes (non-BaseType lines)
const clipboardLines = ref<FilterLine[]>([]);


const filteredBlocks = computed(() => {
    if (!searchQuery.value) return parsedBlocks.value;
    const lower = searchQuery.value.toLowerCase();
    return parsedBlocks.value.filter(b => {
      // Search in header/metadata (name, category, notes/rawHeader)
      const headerMatch = (b.name || '').toLowerCase().includes(lower) || 
                (b.category || '').toLowerCase().includes(lower) ||
                (b.rawHeader || '').toLowerCase().includes(lower);
                            
      if (headerMatch) return true;

      // Search in BaseType/Class content and notes (rawHeader) for completeness
      const contentMatch = b.lines.some(l => {
        const k = l.key.toLowerCase();
        if (k === 'basetype' || k === 'class') {
          return l.values.some(v => {
            const cleanVal = v.replace(/"/g, '').toLowerCase();
            return cleanVal.includes(lower);
          });
        }
        return false;
      }) || (b.rawHeader || '').toLowerCase().includes(lower);
        
      return contentMatch;
    });
});

const getBlockKey = (block: FilterBlock) => {
    // Return unique ID to prevent collisions between identical blocks
    return block.id;
};

const isBlockExpanded = (block: FilterBlock) => {
    if (!selectedFile.value) return false;
    const path = selectedFile.value.path;
    const state = globalViewState[path];
    return state ? state.expandedKey === block.id : false;
};

const handleBlockToggle = async (block: FilterBlock, expanded: boolean) => {
    if (!selectedFile.value) return;
    const path = selectedFile.value.path;
    
    if (!globalViewState[path]) {
        globalViewState[path] = { expandedKey: null, scrollY: 0 };
    }
    
    if (expanded) {
        globalViewState[path].expandedKey = block.id;
         // Also update focused block to this one if expanded
        focusedBlockId.value = block.id;
        
        // Wait for DOM to update (collapse others, expand this one)
        await nextTick();
        
        // Scroll the active block into view
        const el = document.getElementById(`block-${block.id}`);
        if (el) {
            el.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    } else {
        if (globalViewState[path].expandedKey === block.id) {
             globalViewState[path].expandedKey = null;
        }
    }
};

const saveVisualScroll = () => {
    if (selectedFile.value && visualListRef.value) {
        const path = selectedFile.value.path;
        if (!globalViewState[path]) {
            globalViewState[path] = { expandedKey: null, scrollY: 0 };
        }
        globalViewState[path].scrollY = visualListRef.value.scrollTop;
    }
};

const restoreVisualScroll = async () => {
    if (selectedFile.value && globalViewState[selectedFile.value.path]) {
        await nextTick();
        if (visualListRef.value) {
            visualListRef.value.scrollTop = globalViewState[selectedFile.value.path].scrollY;
        }
    }
};

// Preserve block IDs across mode switches to keep expansion/focus state
const reuseBlockIds = (prev: FilterBlock[], next: FilterBlock[]) => {
  const makeSig = (b: FilterBlock) => {
    const linesSig = b.lines
      .map(l => `${l.key}:${l.operator ?? ''}:${l.values.join(' ')}`)
      .join(';');
    return `${b.rawHeader}|${b.type}|${linesSig}`;
  };

  // Allow duplicate signatures by storing queues of IDs
  const bucket = new Map<string, string[]>();
  prev.forEach(b => {
    const sig = makeSig(b);
    const arr = bucket.get(sig) ?? [];
    arr.push(b.id);
    bucket.set(sig, arr);
  });

  next.forEach(b => {
    const sig = makeSig(b);
    const arr = bucket.get(sig);
    if (arr && arr.length) {
      b.id = arr.shift() as string;
    }
  });

  return next;
};

// File Context Menu & Operations
const fileContextMenu = reactive({
    visible: false,
    x: 0,
    y: 0,
    targetFile: null as FilterFile | null
});

// Global handler reference to ensure proper cleanup
let activeCloseMenu: (() => void) | null = null;

const onFileContextMenu = (event: MouseEvent, file: FilterFile) => {
    // Cleanup previous menu listeners immediately
    if (activeCloseMenu) {
        window.removeEventListener('click', activeCloseMenu);
        window.removeEventListener('contextmenu', activeCloseMenu);
        activeCloseMenu = null;
    }

    fileContextMenu.x = event.clientX;
    fileContextMenu.y = event.clientY;
    fileContextMenu.targetFile = file;
    fileContextMenu.visible = true;

    // Define new close handler
    const closeMenu = () => {
        fileContextMenu.visible = false;
        if (activeCloseMenu) {
             window.removeEventListener('click', activeCloseMenu);
             window.removeEventListener('contextmenu', activeCloseMenu);
             activeCloseMenu = null;
        }
    };
    
    activeCloseMenu = closeMenu;

    requestAnimationFrame(() => {
        window.addEventListener('click', closeMenu);
        window.addEventListener('contextmenu', closeMenu);
    });
};

const promptDelete = async () => {
    // 1. Hide menu immediately
    fileContextMenu.visible = false;
    
    // Cleanup listeners since we force closed it
    if (activeCloseMenu) {
        window.removeEventListener('click', activeCloseMenu);
        window.removeEventListener('contextmenu', activeCloseMenu);
        activeCloseMenu = null;
    }

    if (!fileContextMenu.targetFile) return;
    const targetFile = fileContextMenu.targetFile;

    try {
        const yes = await ask(`确定要删除过滤器 "${targetFile.name}" 吗？\n此操作无法撤销。`, {
            title: '确认删除',
            kind: 'warning',
            okLabel: '删除',
            cancelLabel: '取消'
        });

        if (!yes) return;

        await invoke("delete_filter_file", { path: targetFile.path });
        
        // If deleted file was selected, clear selection
        if (selectedFile.value?.path === targetFile.path) {
            selectedFile.value = null;
            parsedBlocks.value = [];
        }
        
        await scanFilters();
    } catch (e) {
        console.error("Delete operation failed", e);
        // Note: ask() might check for permission errors locally on some capabilities but usually works
        alert(`操作失败: ${e}`);
    }
};

const openCreateDialog = async () => {
    const inputName = prompt("请输入新过滤器名称 (.filter):", "NewFilter");
    if (!inputName || !inputName.trim()) return;

    // Ensure .filter extension
    let name = inputName.trim();
    if (!name.toLowerCase().endsWith('.filter')) {
        name += '.filter';
    }
    
    const settings = configManager.getSettings();
    if (!settings.filterStoragePath) {
         alert("请先配置过滤器存储路径");
         return;
    }
    
    // Construct full path
    const path = `${settings.filterStoragePath}\\${name}`;
    
    try {
        // Create empty filter or simple template
        const template = `# Type: Custom\n# Name: ${name}\n\nShow\n    SetFontSize 32\n    SetBorderColor 255 255 255\n`;
        await invoke("write_file_content", { path, content: template });
        
        await scanFilters();
        
        // Auto select new file
        const newFile = filterFiles.value.find(f => f.name === name);
        if (newFile) {
            selectFile(newFile);
        }
    } catch (e) {
        alert(`创建失败: ${e}`);
    }
};

const scanFilters = async () => {
  const settings = configManager.getSettings();
  if (!settings.filterStoragePath) {
    alert("请先在设置中配置过滤器存储路径");
    return;
  }


  isLoading.value = true;
  try {
    const files = await invoke<string[]>("scan_filter_files", {
      path: settings.filterStoragePath,
    });
    
    filterFiles.value = files
      .map((path) => {
        const name = path.split(/[/\\]/).pop() || path;
        return { name, path };
      })
      .sort((a, b) => a.name.localeCompare(b.name));
      
    // If there is a remembered selection, try to restore it
    const settingsAfter = configManager.getSettings();
    if (settingsAfter.lastSelectedFilter) {
      const match = filterFiles.value.find(f => f.path === settingsAfter.lastSelectedFilter);
      if (match) {
        // restore selection but do not re-save setting
        await selectFile(match);
      }
    }
      
  } catch (error) {
    console.error("Failed to scan filters:", error);
    alert(`扫描失败: ${error}`);
  } finally {
    isLoading.value = false;
  }
};

const selectFile = async (file: FilterFile) => {
  if (selectedFile.value && currentViewMode.value === 'visual') {
      saveVisualScroll();
  }
  
  selectedFile.value = file;
  searchQuery.value = ""; // Reset search on file change
  // persist the last selected file
  await configManager.saveSettings({ lastSelectedFilter: file.path });
  isLoading.value = true;
  saveStatus.value = "";
  parsedBlocks.value = [];
  try {
    const content = await invoke<string>("read_file_content", {
      path: file.path,
    });
    rawContent.value = content;
    // Parse immediately
    parsedBlocks.value = FilterParser.parse(content);
    
    // Auto-restore view state
    if (globalViewState[file.path]) {
        // Blocks expansion restored effectively by isBlockExpanded computed logic in template
        if (currentViewMode.value === 'visual') {
            await restoreVisualScroll();
        }
    }
    
  } catch (error) {
    console.error("Failed to read file:", error);
    alert(`读取失败: ${error}`);
  } finally {
    isLoading.value = false;
  }
};

const saveFile = async () => {
  if (!selectedFile.value) return;
  
  try {
    let contentToSave = "";
    if (currentViewMode.value === 'visual') {
        saveVisualScroll();
        // Regenerate from blocks
        contentToSave = FilterParser.stringify(parsedBlocks.value);
        // Sync back to raw for consistency if we switch tabs
        rawContent.value = contentToSave; 
    } else {
        contentToSave = rawContent.value;
        // Sync to blocks? Maybe tricky if syntax error.
        // Let's try to parse silently to update visual view if valid
        try {
            parsedBlocks.value = FilterParser.parse(contentToSave);
        } catch(e) { /* ignore */ }
    }

    await invoke("write_file_content", {
      path: selectedFile.value.path,
      content: contentToSave,
    });
    saveStatus.value = "保存成功!";
    setTimeout(() => saveStatus.value = "", 3000);
  } catch (error) {
    console.error("Failed to save file:", error);
    alert(`保存失败: ${error}`);
  }
};

const openContainingFolder = async () => {
  if (!selectedFile.value) return;
  try {
    // get directory by stripping last segment
    const dir = selectedFile.value.path.replace(/[/\\][^/\\]+$/, '');
    await invoke('open_folder_cmd', { path: dir });
  } catch (e) {
    console.error('Failed to open containing folder', e);
    alert(`打开失败: ${e}`);
  }
};

const codeEditor = ref<HTMLTextAreaElement | null>(null);

const switchMode = async (mode: 'visual' | 'code') => {
  if (mode === currentViewMode.value) return;
  
  // Save current state before switching
  if (currentViewMode.value === 'visual') {
      saveVisualScroll();
  }
    
  if (mode === 'visual') {
    // Switching to visual: Parse current raw text and reuse IDs to retain expansion state
    const newlyParsed = FilterParser.parse(rawContent.value);
    parsedBlocks.value = reuseBlockIds(parsedBlocks.value, newlyParsed);
    await restoreVisualScroll();
  } else {
    // Switching to code: Stringify current blocks
    rawContent.value = FilterParser.stringify(parsedBlocks.value);
    
    // Using setTimeout instead of nextTick due to potential layout thrashing/delays in rendering large textareas
    setTimeout(() => {
        try {
            const textarea = codeEditor.value;
            // Focus on the block that is currently expanded (or focused)
            let targetBlockId = focusedBlockId.value;
            
            // If we have an expanded key, prefer that block
            if (selectedFile.value && globalViewState[selectedFile.value.path]?.expandedKey) {
                const key = globalViewState[selectedFile.value.path].expandedKey;
                const block = parsedBlocks.value.find(b => getBlockKey(b) === key);
                // Only override if found
                if (block) targetBlockId = block.id;
            }

            if (textarea && targetBlockId) {
                const block = parsedBlocks.value.find(b => b.id === targetBlockId);
                
                if (block && typeof block.startLine === 'number') {
                    
                    let lineIndex = block.startLine;

                    // Refinement: Try to find 'BaseType' within the next few lines
                    // to land exactly on the content rather than the header
                    const lines = rawContent.value.split('\n');
                    const limit = Math.min(lines.length, lineIndex + 15);
                    for(let i = lineIndex; i < limit; i++) {
                        if (lines[i].trim().startsWith('BaseType')) {
                            lineIndex = i;
                            break;
                        }
                    }

                    // Calculate character offset manually
                    // We generated the string with '\n', so +1 per line is correct
                    let charIndex = 0;
                    for(let i=0; i < lineIndex; i++) {
                        if (i < lines.length) {
                            charIndex += lines[i].length + 1; 
                        }
                    }
                    
                    textarea.focus();
                    textarea.setSelectionRange(charIndex, charIndex);
                    
                    // Scroll logic based on lines
                    const totalLines = lines.length;
                    const ratio = lineIndex / Math.max(1, totalLines);
                    
                    // Calculate target pixel position
                    // Improve accuracy by centering the target line
                    const targetScroll = (textarea.scrollHeight * ratio) - (textarea.clientHeight / 2);
                    
                    textarea.scrollTop = Math.max(0, targetScroll);
                }
            }
        } catch (e) {
            console.warn('Auto-scroll to block failed', e);
        }
    }, 100);
  }
  currentViewMode.value = mode;
}

const addNewBlock = () => {
    parsedBlocks.value.unshift({
        id: crypto.randomUUID(),
        type: 'Show',
        name: 'New Rule',
        category: 'Custom',
        priority: 'Normal',
        startLine: 0, // Default to top, will be recalculated on save/view switch
        rawHeader: 'Custom - New Rule - Normal',
        lines: []
    });
};

// Context Menu Logic
const contextMenu = reactive<{
    visible: boolean;
    x: number;
    y: number;
    targetBlockId: string | null;
}>({
    visible: false,
    x: 0,
    y: 0,
    targetBlockId: null
});

// Copy/paste helpers (exclude BaseType to respect rule identity)
const cloneNonBaseLines = (block: FilterBlock): FilterLine[] => {
  return block.lines
    .filter((l) => l.key.toLowerCase() !== 'basetype')
    .map((l) => ({ ...l, values: [...l.values] }));
};

const copyBlockAttributes = (blockId: string) => {
  const source = parsedBlocks.value.find((b) => b.id === blockId);
  if (!source) {
    contextMenu.visible = false;
    return;
  }
  clipboardLines.value = cloneNonBaseLines(source);
  contextMenu.visible = false;
};

const pasteBlockAttributes = (blockId: string) => {
  if (!clipboardLines.value.length) {
    contextMenu.visible = false;
    return;
  }
  const target = parsedBlocks.value.find((b) => b.id === blockId);
  if (!target) {
    contextMenu.visible = false;
    return;
  }
  const baseLines = target.lines
    .filter((l) => l.key.toLowerCase() === 'basetype')
    .map((l) => ({ ...l, values: [...l.values] }));
  const pasted = clipboardLines.value.map((l) => ({ ...l, values: [...l.values] }));
  target.lines = [...baseLines, ...pasted];
  contextMenu.visible = false;
};

const createBlockFromAttributes = (blockId: string) => {
  const source = parsedBlocks.value.find((b) => b.id === blockId);
  if (!source) {
    contextMenu.visible = false;
    return;
  }

  const newBlock: FilterBlock = {
    id: crypto.randomUUID(),
    type: source.type,
    startLine: 0,
    category: source.category,
    name: source.name,
    priority: source.priority,
    rawHeader: source.rawHeader,
    lines: cloneNonBaseLines(source)
  };

  parsedBlocks.value.unshift(newBlock);
  contextMenu.visible = false;
};

const onBlockContextMenu = (event: MouseEvent, blockId: string) => {
    // Prevent default browser menu
    // (Handled by .prevent in child, but good to ensure logic here)
    
    contextMenu.x = event.clientX;
    contextMenu.y = event.clientY;
    contextMenu.targetBlockId = blockId;
    contextMenu.visible = true;
    
    // Close on next interaction
    const closeMenu = () => {
        contextMenu.visible = false;
        window.removeEventListener('click', closeMenu);
        window.removeEventListener('contextmenu', closeMenu); // Close if another right click happens
    };
    
    // Delay adding listeners to strictly avoid current event triggering close
    requestAnimationFrame(() => {
        window.addEventListener('click', closeMenu);
    });
};

const deleteTargetBlock = () => {
    if (contextMenu.targetBlockId) {
        const idx = parsedBlocks.value.findIndex(b => b.id === contextMenu.targetBlockId);
        if (idx !== -1) {
            parsedBlocks.value.splice(idx, 1);
        }
    }
    contextMenu.visible = false;
};

const initAndScan = async () => {
  await configManager.init();
  const settings = configManager.getSettings();
  if(settings.filterStoragePath) {
    await scanFilters();
  }
};

onMounted(async () => {
  await initAndScan();
});

onActivated(async () => {
    if (filterFiles.value.length === 0) {
        await initAndScan();
    }
});
</script>

<template>
  <div class="filter-page">
    <!-- Sidebar -->
    <div class="sidebar glass-panel">
      <div class="sidebar-header">
        <h3>过滤器列表</h3>
        <div style="display: flex; gap: 4px;">
            <button class="glass-button small icon" @click="openCreateDialog" title="新建过滤器">+</button>
            <button class="glass-button small icon" @click="scanFilters" title="刷新">↻</button>
        </div>
      </div>
      <div class="file-list">
        <div 
          v-for="file in filterFiles" 
          :key="file.path"
          class="file-item"
          :class="{ active: selectedFile?.path === file.path }"
          @click="selectFile(file)"
          @contextmenu.prevent.stop="onFileContextMenu($event, file)"
        >
          <span class="file-icon">📄</span>
          <span class="file-name">{{ file.name }}</span>
        </div>
      </div>
    </div>

    <!-- Main Content -->
    <div class="main-content glass-panel" v-if="selectedFile">
      <!-- Toolbar -->
      <div class="editor-header">
           <div class="left-tools">
               <span class="current-file">{{ selectedFile.name }}</span>
               <div class="mode-switch">
                   <button 
                    class="glass-button small" 
                    :class="{ active: currentViewMode === 'visual' }"
                    @click="switchMode('visual')">可视化</button>
                   <button 
                    class="glass-button small" 
                    :class="{ active: currentViewMode === 'code' }"
                    @click="switchMode('code')">代码</button>
               </div>
           </div>
           
           <div class="right-tools">
               <input v-if="currentViewMode === 'visual'" v-model="searchQuery" class="glass-input search-box" placeholder="🔍 搜索规则..." />
               <span class="save-status">{{ saveStatus }}</span>
               <button v-if="currentViewMode === 'visual'" class="glass-button" @click="addNewBlock">+ 添加规则</button>
               <button class="glass-button" @click="openContainingFolder">打开所在文件夹</button>
               <button class="glass-button primary" @click="saveFile">保存文件</button>
           </div>
      </div>

      <!-- VISUAL EDITOR -->
      <div v-show="currentViewMode === 'visual'" class="visual-editor-container" ref="visualListRef" @scroll="saveVisualScroll">
          <div class="blocks-list">
              <FilterRuleEditor 
                v-for="block in filteredBlocks" 
                :key="block.id"
                :id="'block-' + block.id"
                :block="block"
                :expanded="isBlockExpanded(block)"
                :filterPath="selectedFile?.path"
                @update:expanded="(val) => handleBlockToggle(block, val)"
                @open-ctx-menu="(e) => onBlockContextMenu(e, block.id)"
                @focus="focusedBlockId = block.id"
              />
              <div v-if="filteredBlocks.length === 0" class="empty-blocks">
                  {{ searchQuery ? '未找到匹配规则' : '没有找到规则，或解析失败。请尝试切换到代码模式查看。' }}
              </div>
          </div>
      </div>

        <!-- CODE EDITOR -->
        <textarea 
          ref="codeEditor"
          v-show="currentViewMode === 'code'"
          v-model="rawContent" 
          class="code-editor" 
          spellcheck="false"
        ></textarea>
     
     <!-- Custom Context Menu -->
     <div v-if="contextMenu.visible" class="context-menu" :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }">
      <div class="context-menu-item" @click.stop="contextMenu.targetBlockId && copyBlockAttributes(contextMenu.targetBlockId)">
        <span>📄 复制属性（不含 BaseType）</span>
      </div>
      <div 
        class="context-menu-item" 
        :class="{ disabled: !clipboardLines.length }"
        @click.stop="contextMenu.targetBlockId && clipboardLines.length && pasteBlockAttributes(contextMenu.targetBlockId)">
        <span>📋 粘贴属性（不含 BaseType）</span>
      </div>
      <div class="context-menu-item" @click.stop="contextMenu.targetBlockId && createBlockFromAttributes(contextMenu.targetBlockId)">
        <span>➕ 以此规则属性创建新规则</span>
      </div>
        <div class="context-menu-item danger" @click.stop="deleteTargetBlock">
            <span>🗑️ 删除规则 (Delete)</span>
        </div>
     </div>

    </div>

    <div v-else class="welcome-state glass-panel main-content">
        <p>请选择左侧文件</p>
    </div>

    <!-- File Context Menu -->
     <div v-if="fileContextMenu.visible" class="context-menu" :style="{ top: fileContextMenu.y + 'px', left: fileContextMenu.x + 'px' }">
        <div class="context-menu-item danger" @click.stop="promptDelete">
            <span>🗑️ 删除文件 (Delete)</span>
        </div>
     </div>

  </div>
</template>

<style scoped>
.filter-page {
  display: flex;
  height: 100%;
  width: 100%;
  gap: 16px;
  padding: 16px;
  box-sizing: border-box;
  color: #eee;
}

.glass-panel {
  background: rgba(30, 30, 30, 0.6);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
}

.sidebar { width: 220px; flex-shrink: 0; }
.main-content { flex: 1; overflow: hidden; }

.sidebar-header, .editor-header {
  padding: 10px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.left-tools, .right-tools {
    display: flex;
    align-items: center;
    gap: 12px;
}

.file-list { flex: 1; overflow-y: auto; padding: 8px; }

.file-item {
  padding: 8px;
  border-radius: 4px;
  cursor: pointer;
  color: #ccc;
  display: flex; align-items: center; gap: 8px;
  font-size: 13px;
}
.file-item:hover { background: rgba(255,255,255,0.1); }
.file-item.active { background: rgba(64,158,255,0.2); color: #409eff; }

.visual-editor-container {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
}

.code-editor {
    flex: 1;
    background: rgba(0,0,0,0.3);
    color: #d4d4d4;
    border: none;
    padding: 16px;
    font-family: monospace;
    resize: none;
    outline: none;
}

.mode-switch {
    display: flex;
    background: rgba(0,0,0,0.2);
    border-radius: 4px;
    padding: 2px;
}

.mode-switch button {
    border: none;
    background: transparent;
    opacity: 0.6;
}

.mode-switch button.active {
    background: rgba(255,255,255,0.1);
    opacity: 1;
}

/* Reusing glass button styles */
.glass-button {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #eee;
  padding: 5px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.glass-button:hover { background: rgba(255, 255, 255, 0.2); }
.glass-button.primary { background: rgba(64, 158, 255, 0.4); border-color: rgba(64, 158, 255, 0.6); }

.search-box {
    width: 200px;
    padding: 4px 8px;
    font-size: 13px;
    height: 28px;
    box-sizing: border-box;
}

.save-status { color: #67c23a; font-size: 12px; }
.welcome-state { align-items: center; justify-content: center; color: #666; }

.context-menu {
    position: fixed;
    background: #252526;
    border: 1px solid #454545;
    box-shadow: 0 4px 6px rgba(0,0,0,0.3);
    border-radius: 4px;
    z-index: 9999;
    min-width: 140px;
    padding: 4px 0;
}

.context-menu-item {
    padding: 8px 12px;
    cursor: pointer;
    color: #eee;
    font-size: 13px;
    display: flex; align-items: center; gap: 8px;
    transition: background 0.1s;
}

.context-menu-item:hover {
    background: #37373d;
}

.context-menu-item.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.context-menu-item.disabled:hover {
  background: inherit;
}

.context-menu-item.danger {
    color: #f56c6c;
}
.context-menu-item.danger:hover {
    background: rgba(245, 108, 108, 0.1);
}
</style>

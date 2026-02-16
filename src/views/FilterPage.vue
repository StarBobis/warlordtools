<script setup lang="ts">
import { ref, onMounted, onActivated, onUnmounted, nextTick, reactive, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import { configManager } from "../utils/ConfigManager";
import { FilterParser, type FilterBlock, type FilterLine } from '../utils/FilterParser';
import FilterRuleEditor from '../components/FilterRuleEditor.vue';
import FileTreeItem, { type FileNode } from '../components/FileTreeItem.vue';

interface FilterFile {
  name: string;
  path: string;
}

type ContextEntry = {
  name: string;
  path: string;
  type: 'file' | 'dir';
};

// Global View State Persistence (Session-based)
// Maps filePath -> { expandedKey: string | null, scrollY: number }
const globalViewState = reactive<Record<string, { expandedKey: string | null, scrollY: number }>>({});

const filterFiles = ref<FilterFile[]>([]); // Flattened list for logic compatibility if needed
const fileTree = ref<FileNode[]>([]); // Tree structure for display

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

// Drag-reorder state (similar approach as TitleBar long-press drag)
const blockDrag = reactive({
  isDragging: false,
  draggingId: null as string | null,
  pendingId: null as string | null,
  ready: false,
  ghostPos: { x: 0, y: 0 },
  startPos: { x: 0, y: 0 },
  placeholderIndex: -1
});
 
const blockDragDelay = 300; // ms - increased to avoid accidental drags
const blockDragMoveThreshold = 6; // px - require small movement before starting drag
let blockDragTimer: number | null = null;

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

  const draggingLabel = computed(() => {
    const blk = parsedBlocks.value.find(b => b.id === blockDrag.draggingId);
    return blk?.name || blk?.rawHeader || '拖动规则';
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

// Drag helpers
const cancelBlockDragTimer = () => {
  if (blockDragTimer !== null) {
    clearTimeout(blockDragTimer);
    blockDragTimer = null;
  }
};

const onBlockDragStart = (event: MouseEvent, blockId: string) => {
  if (currentViewMode.value !== 'visual') return;
  // strict left button check
  if (event.button !== 0) return;

  // Prevent native selections while arming drag
  event.preventDefault();

  blockDrag.ghostPos = { x: event.clientX, y: event.clientY };
  blockDrag.startPos = { x: event.clientX, y: event.clientY };
  blockDrag.pendingId = blockId;
  blockDrag.ready = false;
  
  cancelBlockDragTimer();
  blockDragTimer = window.setTimeout(() => {
    blockDrag.ready = true;
  }, blockDragDelay);
};

const handleBlockDragMove = (event: MouseEvent) => {
  blockDrag.ghostPos = { x: event.clientX, y: event.clientY };

  // Only react while the primary button is held; if we somehow missed mouseup, force a reset
  const primaryHeld = (event.buttons & 1) === 1;
  if (!primaryHeld) {
    if (blockDrag.isDragging || blockDrag.pendingId) {
      handleBlockDragEnd();
    }
    return;
  }

  if (currentViewMode.value !== 'visual') return;

  // Not yet dragging: check if we should start after delay + movement
  if (!blockDrag.isDragging) {
    if (!blockDrag.pendingId || !blockDrag.ready) return;
    const dx = event.clientX - blockDrag.startPos.x;
    const dy = event.clientY - blockDrag.startPos.y;
    const dist = Math.hypot(dx, dy);
    if (dist < blockDragMoveThreshold) return;

    const idx = parsedBlocks.value.findIndex(b => b.id === blockDrag.pendingId);
    if (idx === -1) return;

    blockDrag.isDragging = true;
    blockDrag.draggingId = blockDrag.pendingId;
    blockDrag.placeholderIndex = idx;
  }

  if (!blockDrag.draggingId) return;

  const visibleBlocks = filteredBlocks.value;
  if (!visibleBlocks.length) return;

  const y = event.clientY;
  const elements: { el: HTMLElement; id: string }[] = [];
  for (const blk of visibleBlocks) {
    const el = document.getElementById(`block-${blk.id}`);
    if (el) elements.push({ el, id: blk.id });
  }
  if (!elements.length) return;

  let targetId = elements[elements.length - 1].id;

  for (let i = 0; i < elements.length; i++) {
    const { el, id } = elements[i];
    const rect = el.getBoundingClientRect();
    const mid = rect.top + rect.height / 2;
    if (y < mid) {
      targetId = id;
      break;
    }
  }

  const draggedIndex = parsedBlocks.value.findIndex(b => b.id === blockDrag.draggingId);
  const targetIndex = parsedBlocks.value.findIndex(b => b.id === targetId);
  if (draggedIndex === -1 || targetIndex === -1 || draggedIndex === targetIndex) return;

  const updated = [...parsedBlocks.value];
  const [item] = updated.splice(draggedIndex, 1);
  updated.splice(targetIndex, 0, item);
  parsedBlocks.value = updated;
  blockDrag.placeholderIndex = targetIndex;
};

const handleBlockDragEnd = () => {
  cancelBlockDragTimer();
  
  if (blockDrag.isDragging) {
      blockDrag.isDragging = false;
      blockDrag.draggingId = null;
      blockDrag.placeholderIndex = -1;
  }

  // Always clear pending state so a quick click does not arm the next drag
  blockDrag.pendingId = null;
  blockDrag.ready = false;
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
  targetEntry: null as ContextEntry | null
});

const fileContextMenuRef = ref<HTMLElement | null>(null);

// Global handler reference to ensure proper cleanup
let activeCloseMenu: (() => void) | null = null;

const onFileContextMenu = (event: MouseEvent, node: FileNode) => {
    // Cleanup previous menu listeners immediately
    if (activeCloseMenu) {
        window.removeEventListener('click', activeCloseMenu);
        window.removeEventListener('contextmenu', activeCloseMenu);
        activeCloseMenu = null;
    }

    fileContextMenu.x = event.clientX;
    fileContextMenu.y = event.clientY;
  fileContextMenu.targetEntry = {
    name: node.name,
    path: node.path,
    type: node.type
  };
    fileContextMenu.visible = true;

    // Boundary check
    nextTick(() => {
        const menuEl = fileContextMenuRef.value;
        const margin = 8;
        const menuW = menuEl?.offsetWidth ?? 160;
        const menuH = menuEl?.offsetHeight ?? 100;
        const maxX = window.innerWidth - menuW - margin;
        const maxY = window.innerHeight - menuH - margin;
        
        fileContextMenu.x = Math.min(fileContextMenu.x, Math.max(margin, maxX));
        fileContextMenu.y = Math.min(fileContextMenu.y, Math.max(margin, maxY));
    });

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

const openContainingFolderForTarget = async () => {
    // Close menu
    fileContextMenu.visible = false;
     if (activeCloseMenu) {
        window.removeEventListener('click', activeCloseMenu);
        window.removeEventListener('contextmenu', activeCloseMenu);
        activeCloseMenu = null;
    }

  if (!fileContextMenu.targetEntry) return;
    try {
    const target = fileContextMenu.targetEntry;
    const dir = target.type === 'file'
      ? target.path.replace(/[/\\][^/\\]+$/, '')
      : target.path;
    await invoke('open_folder_cmd', { path: dir });
    } catch (e) {
        alert(`打开失败: ${e}`);
    }
};

  const promptRename = async () => {
    // Close the context menu first
    fileContextMenu.visible = false;

    if (activeCloseMenu) {
      window.removeEventListener('click', activeCloseMenu);
      window.removeEventListener('contextmenu', activeCloseMenu);
      activeCloseMenu = null;
    }

    if (!fileContextMenu.targetEntry || fileContextMenu.targetEntry.type !== 'file') return;
    const targetFile = fileContextMenu.targetEntry;

    const suggested = targetFile.name.replace(/\.filter$/i, '');
    const inputName = prompt("请输入新的过滤器名称 (.filter):", suggested);
    if (!inputName || !inputName.trim()) return;

    let name = inputName.trim();
    if (!name.toLowerCase().endsWith('.filter')) {
      name += '.filter';
    }

    // Build new path in the same directory
    const dir = targetFile.path.replace(/[/\\][^/\\]+$/, '');
    const newPath = `${dir}\\${name}`;

    // No-op if unchanged
    if (newPath === targetFile.path) return;

    try {
      await invoke("rename_filter_file", { oldPath: targetFile.path, newPath });
      await scanFilters();

      const renamed = filterFiles.value.find(f => f.path === newPath);
      if (renamed) {
        await selectFile(renamed);
      } else if (selectedFile.value?.path === targetFile.path) {
        // Fallback: clear selection if the renamed file isn't found after scan
        selectedFile.value = null;
        parsedBlocks.value = [];
      }
    } catch (e) {
      alert(`重命名失败: ${e}`);
    }
  };

const promptDeleteFile = async () => {
    // 1. Hide menu immediately
    fileContextMenu.visible = false;
    
    // Cleanup listeners since we force closed it
    if (activeCloseMenu) {
        window.removeEventListener('click', activeCloseMenu);
        window.removeEventListener('contextmenu', activeCloseMenu);
        activeCloseMenu = null;
    }

    if (!fileContextMenu.targetEntry || fileContextMenu.targetEntry.type !== 'file') return;
    const targetFile = fileContextMenu.targetEntry;

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

    const promptDeleteFolder = async () => {
      fileContextMenu.visible = false;
      if (activeCloseMenu) {
        window.removeEventListener('click', activeCloseMenu);
        window.removeEventListener('contextmenu', activeCloseMenu);
        activeCloseMenu = null;
      }

      if (!fileContextMenu.targetEntry || fileContextMenu.targetEntry.type !== 'dir') return;
      const targetFolder = fileContextMenu.targetEntry;

      try {
        const yes = await ask(`确定要删除文件夹 "${targetFolder.name}" 及其所有过滤器吗？\n此操作无法撤销。`, {
          title: '确认删除文件夹',
          kind: 'warning',
          okLabel: '删除文件夹',
          cancelLabel: '取消'
        });

        if (!yes) return;

        await invoke('delete_filter_folder', { path: targetFolder.path });

        // Clear selection if current file is inside the deleted folder
        const normalize = (p: string) => p.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();
        const deletedPrefix = normalize(targetFolder.path);
        if (selectedFile.value && normalize(selectedFile.value.path).startsWith(deletedPrefix)) {
          selectedFile.value = null;
          parsedBlocks.value = [];
        }

        await scanFilters();
      } catch (e) {
        console.error('Delete folder failed', e);
        alert(`操作失败: ${e}`);
      }
    };

const openCreateDialog = async () => {
    const inputName = prompt("请输入新过滤器名称 (.filter):", "NewFilter");
    if (!inputName || !inputName.trim()) return;

    // Ensure .filter extension
  let rawName = inputName.trim();
  if (rawName.toLowerCase().endsWith('.filter')) {
    rawName = rawName.slice(0, -7); // remove extension for folder/file base name
  }

  const baseName = rawName || 'NewFilter';
  const fileName = `${baseName}.filter`;
    
    const settings = configManager.getSettings();
    if (!settings.filterStoragePath) {
         alert("请先配置过滤器存储路径");
         return;
    }
    
    // Construct folder + file path
    const sep = settings.filterStoragePath.includes('\\') ? '\\' : '/';
    const folderPath = `${settings.filterStoragePath}${sep}${baseName}`;
    const path = `${folderPath}${sep}${fileName}`;
    
    try {
      // Guard: avoid overwriting existing file
      const exists = await invoke<boolean>('path_exists', { path });
      if (exists) {
        alert('同名过滤器已存在，请使用其他名称');
        return;
      }

      // Ensure folder exists then create file
      await invoke('create_filter_folder', { path: folderPath });

      const template = `# Type: Custom\n# Name: ${fileName}\n\nShow\n    SetFontSize 32\n    SetBorderColor 255 255 255\n`;
      await invoke("write_file_content", { path, content: template });
        
        await scanFilters();
        
        // Auto select new file
      const newFile = filterFiles.value.find(f => f.path === path);
        if (newFile) {
            selectFile(newFile);
        }
    } catch (e) {
        alert(`创建失败: ${e}`);
    }
};


const buildFileTree = (paths: string[], rootPath: string | undefined): FileNode[] => {
    // If no root path, just list files flat (fallback)
    if (!rootPath) {
         return paths.map(p => ({
             name: p.split(/[/\\]/).pop() || p,
             path: p,
             type: 'file',
             children: [],
             expanded: false
         }));
    }

    const rootNodes: FileNode[] = [];
  const normalizedRoot = rootPath.replace(/\\/g, '/');
  const separator = rootPath.includes('\\') ? '\\' : '/';
  const sepForRegex = separator === '\\' ? '\\\\' : separator;
  const trimmedRoot = rootPath.replace(new RegExp(`[${sepForRegex}]+$`), '');

  const buildDirPath = (segments: string[]) => {
    const joined = segments.join(separator);
    return trimmedRoot ? `${trimmedRoot}${separator}${joined}` : joined;
  };

    paths.forEach(fullPath => {
        const normalizedPath = fullPath.replace(/\\/g, '/');
        // Ensure path starts with root
        if (!normalizedPath.toLowerCase().startsWith(normalizedRoot.toLowerCase())) return;
        
        // Get relative part, +1 for slash
        // Handle case where root matching might need care with trailing slash
        let relative = normalizedPath.slice(normalizedRoot.length);
        if (relative.startsWith('/')) relative = relative.slice(1);
        
        const parts = relative.split('/');
        let currentLevel = rootNodes;
        
        const currentParts: string[] = [];
        parts.forEach((part, index) => {
            const isLast = index === parts.length - 1;
          currentParts.push(part);
          const nodePath = isLast ? fullPath : buildDirPath(currentParts);
            const existing = currentLevel.find(n => n.name === part);
            
            if (existing) {
            if (existing.type === 'dir' && !existing.path) {
              existing.path = nodePath;
            }
                if (isLast) {
                    // Should not happen if folder name is same as file name?
                    // But assume folder structure is clean
                } else {
                    currentLevel = existing.children;
                }
            } else {
                const nodeType = isLast ? 'file' : 'dir';
                // Reconstruct path for this node
                // Note: fullPath is only valid for the leaf file.
                // For directories, we need to construct it.
                // But simplified: we only really need path for 'file' types right now for selection.
                // For folders, let's construct it properly just in case
                
                // Construct path up to this part
                // This is a bit tricky with separators logic but let's try
                // Simple approach: parent node path + part
                // Wait, recursive search for path construction is complex.
                // BUT, 'fullPath' contains everything.
                // For intermediate folders, we can deduce.
                // Actually, we can just defer path setting for folders or use what we parsed.
                
                const newNode: FileNode = {
                  name: part,
                  path: nodePath,
                  type: nodeType,
                  children: [],
                  expanded: false
                };
                
                currentLevel.push(newNode);
                
                if (!isLast) {
                    currentLevel = newNode.children;
                }
            }
        });
    });

    // Helper to sort: Folder < File, then Alphabetical
    const sortNodes = (nodes: FileNode[]) => {
        nodes.sort((a, b) => {
            if (a.type !== b.type) {
                return a.type === 'dir' ? -1 : 1;
            }
            return a.name.localeCompare(b.name, undefined, { numeric: true });
        });
        nodes.forEach(n => sortNodes(n.children));
    };
    
    sortNodes(rootNodes);
    return rootNodes;
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
    
    // Update flat list
    filterFiles.value = files
      .map((path) => {
        const name = path.split(/[/\\]/).pop() || path;
        return { name, path };
      })
      .sort((a, b) => a.name.localeCompare(b.name));
      
    // Build tree
    fileTree.value = buildFileTree(files, settings.filterStoragePath);
      
    // If there is a remembered selection, try to restore it
    const settingsAfter = configManager.getSettings();
    if (settingsAfter.lastSelectedFilter) {
      const match = filterFiles.value.find(f => f.path === settingsAfter.lastSelectedFilter);
      if (match) {
        // Find in tree and expand parents?
        // Recursive expand
        const expandPath = (nodes: FileNode[], targetPath: string): boolean => {
            for(const node of nodes) {
                if (node.type === 'file' && node.path === targetPath) return true;
                if (node.type === 'dir') {
                    if (expandPath(node.children, targetPath)) {
                        node.expanded = true;
                        return true;
                    }
                }
            }
            return false;
        };
        expandPath(fileTree.value, match.path);

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

const handleNodeSelect = (node: FileNode) => {
    if (node.type === 'file') {
        const fileObj = filterFiles.value.find(f => f.path === node.path);
        if (fileObj) {
            selectFile(fileObj);
        }
    }
};

const handleNodeToggle = (node: FileNode) => {
    node.expanded = !node.expanded;
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
      rawHeader: '',
      inlineComments: [],
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

const blockContextMenuRef = ref<HTMLElement | null>(null);

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
    inlineComments: [],
    lines: cloneNonBaseLines(source)
  };

  parsedBlocks.value.unshift(newBlock);
  contextMenu.visible = false;
};

const onBlockContextMenu = (event: MouseEvent, blockId: string) => {
  // Prevent default browser menu
  // (Handled by .prevent in child, but keep logic here)

  // Use client coords for fixed positioning
  const margin = 8;
  contextMenu.x = event.clientX;
  contextMenu.y = event.clientY;
  contextMenu.targetBlockId = blockId;
  contextMenu.visible = true;

  nextTick(() => {
    const menuEl = blockContextMenuRef.value;
    const menuW = menuEl?.offsetWidth ?? 180;
    const menuH = menuEl?.offsetHeight ?? 160;
    const maxX = window.innerWidth - menuW - margin;
    const maxY = window.innerHeight - menuH - margin;
    contextMenu.x = Math.min(contextMenu.x, Math.max(margin, maxX));
    contextMenu.y = Math.min(contextMenu.y, Math.max(margin, maxY));
  });

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
  // Attach global listeners in capture phase so they still fire even if inner handlers stop propagation
  window.addEventListener('mousemove', handleBlockDragMove, true);
  window.addEventListener('mouseup', handleBlockDragEnd, true);
  window.addEventListener('pointerup', handleBlockDragEnd, true);
  window.addEventListener('pointercancel', handleBlockDragEnd, true);
  window.addEventListener('mouseleave', handleBlockDragEnd, true);
  window.addEventListener('blur', handleBlockDragEnd);
  await initAndScan();
});

onUnmounted(() => {
  window.removeEventListener('mousemove', handleBlockDragMove, true);
  window.removeEventListener('mouseup', handleBlockDragEnd, true);
  window.removeEventListener('pointerup', handleBlockDragEnd, true);
  window.removeEventListener('pointercancel', handleBlockDragEnd, true);
  window.removeEventListener('mouseleave', handleBlockDragEnd, true);
  window.removeEventListener('blur', handleBlockDragEnd);
  cancelBlockDragTimer();
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
        <FileTreeItem 
            v-for="node in fileTree" 
            :key="node.name + node.path" 
            :node="node" 
            :selectedPath="selectedFile?.path"
            @select="handleNodeSelect"
            @toggle="handleNodeToggle"
          @context-menu="(e, n) => onFileContextMenu(e, n)"
        />
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
                :class="{ dragging: blockDrag.draggingId === block.id }"
                :block="block"
                :expanded="isBlockExpanded(block)"
                :filterPath="selectedFile?.path"
                @update:expanded="(val) => handleBlockToggle(block, val)"
                @open-ctx-menu="(e) => onBlockContextMenu(e, block.id)"
                @start-drag="(e, id) => onBlockDragStart(e, id)"
                @focus="focusedBlockId = block.id"
              />
              <div v-if="filteredBlocks.length === 0" class="empty-blocks">
                  {{ searchQuery ? '未找到匹配规则' : '没有找到规则，或解析失败。请尝试切换到代码模式查看。' }}
              </div>
          </div>

          <div v-if="blockDrag.isDragging && blockDrag.draggingId" class="block-ghost" :style="{ left: blockDrag.ghostPos.x + 'px', top: blockDrag.ghostPos.y + 'px' }">
            拖动：{{ draggingLabel }}
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
     <Teleport to="body">
       <div
        v-if="contextMenu.visible"
        ref="blockContextMenuRef"
        class="context-menu glass-menu"
        :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
       >
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
     </Teleport>

    </div>

    <div v-else class="welcome-state glass-panel main-content">
        <p>请选择左侧文件</p>
    </div>

    <!-- File Context Menu -->
     <Teleport to="body">
       <div 
        v-if="fileContextMenu.visible" 
        ref="fileContextMenuRef"
        class="context-menu glass-menu" 
        :style="{ top: fileContextMenu.y + 'px', left: fileContextMenu.x + 'px' }"
       >
        <div class="context-menu-item" @click.stop="openContainingFolderForTarget">
          <span>{{ fileContextMenu.targetEntry?.type === 'file' ? '📂 打开所在文件夹' : '📂 打开此文件夹' }}</span>
        </div>
        <div 
          v-if="fileContextMenu.targetEntry?.type === 'file'" 
          class="context-menu-item" 
          @click.stop="promptRename"
        >
          <span>✏️ 重命名文件</span>
        </div>
          <div 
            v-if="fileContextMenu.targetEntry?.type === 'file'" 
            class="context-menu-item danger" 
            @click.stop="promptDeleteFile"
          >
              <span>🗑️ 删除文件 (Delete)</span>
          </div>
          <div 
            v-else 
            class="context-menu-item danger" 
            @click.stop="promptDeleteFolder"
          >
              <span>🗑️ 删除此文件夹</span>
          </div>
       </div>
     </Teleport>

  </div>
</template>

<style scoped>
/* Global fix for context menus in body */
:global(.glass-menu) {
    position: fixed;
    background: rgba(40, 40, 40, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(12px);
    border-radius: 8px;
    z-index: 99999;
    min-width: 180px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

:global(.glass-menu .context-menu-item) {
    padding: 8px 12px;
    cursor: pointer;
    color: #e0e0e0;
    font-size: 13px;
    display: flex; 
    align-items: center; 
    gap: 10px;
    border-radius: 4px;
    transition: all 0.15s ease;
    user-select: none;
}

:global(.glass-menu .context-menu-item:hover) {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
    transform: translateX(2px);
}

:global(.glass-menu .context-menu-item.disabled) {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}

:global(.glass-menu .context-menu-item.danger) {
    color: #ff6b6b;
}
:global(.glass-menu .context-menu-item.danger:hover) {
    background: rgba(255, 107, 107, 0.15);
}

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

:deep(.filter-block-card.dragging) {
  opacity: 0.5;
  transform: scale(0.99);
}

.block-ghost {
  position: fixed;
  pointer-events: none;
  z-index: 9999;
  background: rgba(40, 44, 52, 0.9);
  border: 1px solid rgba(255,255,255,0.2);
  color: #fff;
  padding: 6px 12px;
  border-radius: 4px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.45);
  transform: translate(-50%, -50%);
  font-size: 12px;
  backdrop-filter: blur(4px);
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
</style>

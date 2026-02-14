<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import type { FilterBlock, FilterLine } from '../utils/FilterParser';

const props = defineProps<{
  block: FilterBlock;
  expanded?: boolean;
}>();

const emit = defineEmits<{
    (e: 'delete'): void;
    (e: 'focus', id: string): void;
    (e: 'update:expanded', val: boolean): void;
    (e: 'open-ctx-menu', event: MouseEvent): void;
}>();

const isExpanded = computed({
  get: () => props.expanded || false,
  set: (val) => emit('update:expanded', val)
});

const onContextMenu = (event: MouseEvent) => {
    emit('open-ctx-menu', event);
};

const localBlock = ref<FilterBlock>(props.block);

// Helper to get/set specific properties efficiently
const findLineIndex = (key: string) => {
    return localBlock.value.lines.findIndex(l => l.key.toLowerCase() === key.toLowerCase());
};

const getLineValue = (key: string): string => {
  const line = localBlock.value.lines.find(l => l.key.toLowerCase() === key.toLowerCase());
  if (!line) return '';
  // Combine multiple lines? Usually standard filters only have one line per key 
  // EXCEPT for BaseType/Class where they list multiple values.
  
  // Remove quotes for display
  return line.values.map(v => v.replace(/"/g, '')).join(' ');
};

const setLineValue = (key: string, val: string, needsQuotes = false) => {
  // If val is empty, remove the line
  if (!val.trim()) {
    localBlock.value.lines = localBlock.value.lines.filter(l => l.key.toLowerCase() !== key.toLowerCase());
    return;
  }

  let values: string[] = [];

  if (needsQuotes) {
      // Logic for quoted strings (Class, BaseType etc)
      
      // Heuristic: If the user typed actual quotes, use regex to capture them.
      // If no quotes found, split by commas or newlines.
      if (val.includes('"')) {
          const matches = val.match(/"[^"]*"|[^,\s]+/g);
          if (matches) {
              values = matches.map(m => {
                  const raw = m.replace(/^"|"$/g, '').trim();
                  return `"${raw}"`;
              });
          }
      } else {
          // No quotes typed? Treat as comma/newline separated list of strings that NEED quotes
          values = val.split(/,|，|\n/).map(s => s.trim()).filter(s => s).map(s => `"${s}"`);
      }
  } else {
      // Logic for numeric/simple values (Colors, Sounds)
      // Remove commands/separators
      // Use split(' ') to allow typing spaces without them being eaten
      values = val.replace(/,/g, ' ').split(' ');
  }

  const existingIndex = findLineIndex(key);
  
  const newLine: FilterLine = {
    key, // Use canonical key casing when saving
    operator: needsQuotes ? undefined : undefined, 
    values,
    raw: '' 
  };

  if (existingIndex >= 0) {
    // Preserve operator if existing
    const oldOp = localBlock.value.lines[existingIndex].operator;
    if(oldOp && ['=', '==', '>', '<', '>=', '<='].includes(oldOp)) {
        newLine.operator = oldOp;
    }
    localBlock.value.lines[existingIndex] = newLine;
  } else {
    localBlock.value.lines.push(newLine);
  }
};

// Computed Properties for common fields

// BaseType needs special handling for large text areas and multiple delimiters
const baseTypes = computed({
  get: () => {
      const line = localBlock.value.lines.find(l => l.key.toLowerCase() === 'basetype');
      if (!line) return '';
      return line.values.map(v => v.replace(/^"|"$/g, '')).join(', ');
  },
  set: (val) => {
      // Smart split: Handle newlines, commas (both English and Chinese), and manual quotes
      let values: string[] = [];
      
      if (val.includes('"')) {
          const matches = val.match(/"[^"]*"|[^,\s\n]+/g);
          if (matches) {
              values = matches.map(m => `"${m.replace(/^"|"$/g, '')}"`);
          }
      } else {
          const rawValues = val.split(/,|，|\n/).map(s => s.trim()).filter(s => s);
          values = rawValues.map(s => `"${s}"`);
      }
      
      const idx = findLineIndex('BaseType');
      if (idx >= 0) {
          if (values.length === 0) {
               localBlock.value.lines.splice(idx, 1);
          } else {
              localBlock.value.lines[idx].values = values;
              localBlock.value.lines[idx].key = 'BaseType'; // Ensure casing
          }
      } else if (values.length > 0) {
          localBlock.value.lines.push({
              key: 'BaseType',
              operator: undefined, 
              values,
              raw: ''
          });
      }
  }
});

const baseTypeInput = ref<HTMLTextAreaElement | null>(null);

const adjustTextareaHeight = () => {
    const el = baseTypeInput.value;
    if (el) {
        el.style.height = 'auto'; // Shrink to fit content first
        el.style.height = (el.scrollHeight + 2) + 'px'; // Expand to needed height
    }
};

watch(baseTypes, () => {
    nextTick(adjustTextareaHeight);
});

watch(isExpanded, (val) => {
    if(val) {
        // Need a slight delay or nextTick for v-if to render the element
        nextTick(() => {
            adjustTextareaHeight();
        });
        // Extra safety delay for transitions
        setTimeout(adjustTextareaHeight, 100);
    }
});


const itemClass = computed({
    get: () => getLineValue('Class'),
    set: (v) => setLineValue('Class', v, true)
});

const alertSoundId = computed({
    get: () => {
        const val = getLineValue('PlayAlertSound');
        if (!val) return '';
        const parts = val.trim().split(' ');
        return parts[0] || '';
    },
    set: (newId) => {
        const current = getLineValue('PlayAlertSound');
        const parts = current ? current.trim().split(' ') : [];
        let vol = '50';
        if (parts.length >= 2) {
            vol = parts[1];
        }
        
        if (!newId) {
             setLineValue('PlayAlertSound', '');
        } else {
             setLineValue('PlayAlertSound', `${newId} ${vol}`);
        }
    }
});

const alertSoundVolume = computed({
    get: () => {
        const val = getLineValue('PlayAlertSound');
        if (!val) return 50;
        const parts = val.trim().split(' ');
        if (parts.length >= 2) {
             const n = parseInt(parts[1]);
             return isNaN(n) ? 50 : n;
        }
        return 50; // Default to 50 as requested if volume is missing
    },
    set: (v) => {
        // Clamp between 0 and 300
        const newVol = Math.max(0, Math.min(300, v === '' ? 50 : Number(v)));
        
        const current = getLineValue('PlayAlertSound');
        const parts = current ? current.trim().split(' ') : [];
        let id = '1';
        if (parts.length >= 1 && parts[0]) {
            id = parts[0];
        } else {
             // If no ID exists (rule disabled), defaulting to ID 1 to enable it
             id = '1';
        }
        setLineValue('PlayAlertSound', `${id} ${newVol}`);
    }
});

const customAlertSound = computed({
    get: () => getLineValue('CustomAlertSound'),
    set: (val) => {
        if (!val.trim()) {
            localBlock.value.lines = localBlock.value.lines.filter(l => l.key !== 'CustomAlertSound');
            return;
        }
        
        // Heuristic: Check if last part is a volume number (0-300)
        const parts = val.trim().match(/(.*)\s+(\d+)$/);
        
        let file = val.trim();
        let vol = '';
        
        if (parts) {
            file = parts[1];
            vol = parts[2];
        }

        const values = [`"${file.replace(/"/g, '')}"`];
        if (vol) values.push(vol);

        const key = 'CustomAlertSound';
        const idx = localBlock.value.lines.findIndex(l => l.key === key);
        const newLine: FilterLine = { key, values, raw: '' };

        if (idx >= 0) {
            localBlock.value.lines[idx] = newLine;
        } else {
            localBlock.value.lines.push(newLine);
        }
    }
});

const browseSound = async () => {
    try {
        const file = await open({
            multiple: false,
            filters: [{
                name: 'Audio Files',
                extensions: ['mp3', 'wav', 'ogg']
            }]
        });

        if (file && typeof file === 'string') {
            // Use just the filename as standard convention for PoE filters
            // But allow preserving volume if it exists
            const current = customAlertSound.value;
            const currentInfo = current.match(/(.*)\s+(\d+)$/);
            const vol = currentInfo ? ' ' + currentInfo[2] : '';
            
            // Extract basename for cleaner filter file (assuming files are next to filter)
            // If user wants full path, they can edit it manually or we can change this policy.
            // Using basic replacement for cross-platform path separators
            const basename = file.replace(/^.*[\\/]/, '');
            
            customAlertSound.value = basename + vol;
        }
    } catch (err) {
        console.error('Failed to open dialog:', err);
    }
};

const disableDropSound = computed({
    get: () => findLineIndex('DisableDropSound') !== -1,
    set: (v) => {
        if (v) {
            if (findLineIndex('DisableDropSound') === -1) {
                localBlock.value.lines.push({ key: 'DisableDropSound', values: [], raw: '' });
            }
        } else {
            localBlock.value.lines = localBlock.value.lines.filter(l => l.key.toLowerCase() !== 'disabledropsound');
        }
    }
});

const disableDropSoundIfAlertSound = computed({
    get: () => findLineIndex('DisableDropSoundIfAlertSound') !== -1,
    set: (v) => {
        if (v) {
             if (findLineIndex('DisableDropSoundIfAlertSound') === -1) {
                localBlock.value.lines.push({ key: 'DisableDropSoundIfAlertSound', values: [], raw: '' });
            }
        } else {
            localBlock.value.lines = localBlock.value.lines.filter(l => l.key.toLowerCase() !== 'disabledropsoundifalertsound');
        }
    }
});

const shouldContinue = computed({
    get: () => findLineIndex('Continue') !== -1,
    set: (v) => {
        if (v) {
             if (findLineIndex('Continue') === -1) {
                // Continue is typically the last line, but order doesn't strictly matter for parsing, 
                // though conventionally it's at the end.
                localBlock.value.lines.push({ key: 'Continue', values: [], raw: '' });
            }
        } else {
            localBlock.value.lines = localBlock.value.lines.filter(l => l.key.toLowerCase() !== 'continue');
        }
    }
});

// Numeric/Compare Fields
const createNumericComputed = (key: string) => computed({
    get: () => {
        const idx = findLineIndex(key);
        if (idx === -1) return '';
        const line = localBlock.value.lines[idx];
        // Return format: ">= 65" or just "65"
        return (line.operator ? line.operator + ' ' : '') + line.values.join(' ');
    },
    set: (val) => {
        if (!val.trim()) {
            // Remove case-insensitively
            localBlock.value.lines = localBlock.value.lines.filter(l => l.key.toLowerCase() !== key.toLowerCase());
            return;
        }
        // Split operator from value e.g. ">= 65" -> op:">=", val:"65"
        // Regex to separate operator from numbers
        const parts = val.trim().match(/^([<>=!]+)?\s*(.*)$/);
        let operator: string | undefined = undefined;
        let valueStr = val;
        
        if (parts) {
            if (parts[1]) operator = parts[1];
            valueStr = parts[2];
        }

        // Use simple split to preserve spaces while typing
        const values = valueStr.split(' ');
        
        // Update or Add
        const idx = findLineIndex(key);
        const newLine = { key, operator, values, raw: '' };
        
        if (idx >= 0) {
            localBlock.value.lines[idx] = newLine;
        } else {
            localBlock.value.lines.push(newLine);
        }
    }
});

const createBooleanComputed = (key: string) => computed({
    // Returns: 'True' | 'False' | '' (meaning undefined/ignore)
    get: () => {
        const idx = findLineIndex(key);
        if (idx === -1) return '';
        const line = localBlock.value.lines[idx];
        
        // If the line exists but has no values, treat as True (legacy behavior fallback)
        if (line.values.length === 0) return 'True';
        
        if (line.values.includes('True')) return 'True';
        if (line.values.includes('False')) return 'False';
        return '';
    },
    set: (val: string) => {
        if (!val) {
            localBlock.value.lines = localBlock.value.lines.filter(l => l.key.toLowerCase() !== key.toLowerCase());
        } else {
            const idx = findLineIndex(key);
            // Always set explicit True/False for clarity and compatibility
            const newLine = { key, values: [val], raw: '' };
            if (idx >= 0) localBlock.value.lines[idx] = newLine;
            else localBlock.value.lines.push(newLine);
        }
    }
});

// Basic Conditions
const itemLevel = createNumericComputed('ItemLevel');
const dropLevel = createNumericComputed('DropLevel');
const quality = createNumericComputed('Quality');
const rarity = createNumericComputed('Rarity'); // Enhanced to support >= Rare
const stackSize = createNumericComputed('StackSize');
const width = createNumericComputed('Width');
const height = createNumericComputed('Height');

// Sockets
const sockets = createNumericComputed('Sockets');
const linkedSockets = createNumericComputed('LinkedSockets');
// SocketGroup can be complex: "RRG", "2R 2G", etc.
// It is NOT quoted.
const socketGroup = computed({
    get: () => getLineValue('SocketGroup'), // e.g. "3G" or "4RGB"
    set: (v) => setLineValue('SocketGroup', v, false) // No quotes
});

// Map Info
const mapTier = createNumericComputed('MapTier');
const areaLevel = createNumericComputed('AreaLevel');
const blightedMap = createBooleanComputed('BlightedMap');
const uberBlightedMap = createBooleanComputed('UberBlightedMap');
const shapedMap = createBooleanComputed('ShapedMap');
const elderMap = createBooleanComputed('ElderMap');

// Gem Info
const gemLevel = createNumericComputed('GemLevel');
const transfiguredGem = computed({
    // TransfiguredGem can be boolean (True/False) OR string ("Leap Slam")
    // For simplicity, treat as string input
    get: () => getLineValue('TransfiguredGem'),
    set: (v) => setLineValue('TransfiguredGem', v, true) // Needs quotes if name
});

// Influence & Status
const identified = createBooleanComputed('Identified');
const corrupted = createBooleanComputed('Corrupted');
const corruptedMods = createNumericComputed('CorruptedMods');
const mirrored = createBooleanComputed('Mirrored');
const fracturedItem = createBooleanComputed('FracturedItem');
const synthesisedItem = createBooleanComputed('SynthesisedItem');
const replica = createBooleanComputed('Replica');
const elderItem = createBooleanComputed('ElderItem');
const shaperItem = createBooleanComputed('ShaperItem');
const hasInfluence = computed({
    get: () => getLineValue('HasInfluence'),
    set: (v) => setLineValue('HasInfluence', v)
});
const hasImplicitMod = createBooleanComputed('HasImplicitMod');
const hasExplicitMod = computed({
    get: () => getLineValue('HasExplicitMod'), // Complex: >= 1 "ModA"
    set: (v) => setLineValue('HasExplicitMod', v, true) 
});

const anyEnchantment = createBooleanComputed('AnyEnchantment');
const hasEnchantment = computed({
    get: () => getLineValue('HasEnchantment'),
    set: (v) => setLineValue('HasEnchantment', v, true)
});
// Removed duplicate declaration of hasImplicitMod

const hasEaterOfWorldsImplicit = createNumericComputed('HasEaterOfWorldsImplicit');
const hasSearingExarchImplicit = createNumericComputed('HasSearingExarchImplicit');
// baseDefencePercentile already defined as baseDefencePct below, avoiding duplicate
// const baseDefencePercentile = createNumericComputed('BaseDefencePercentile'); 

// Base Stats
const baseArmour = createNumericComputed('BaseArmour');
const baseEvasion = createNumericComputed('BaseEvasion');
const baseES = createNumericComputed('BaseEnergyShield');
const baseWard = createNumericComputed('BaseWard');
const baseDefencePct = createNumericComputed('BaseDefencePercentile');

// Cluster Jewels
const enchantmentPassiveNode = computed({
    get: () => getLineValue('EnchantmentPassiveNode'),
    set: (v) => setLineValue('EnchantmentPassiveNode', v, true) // Needs quotes
});
const enchantmentPassiveNum = createNumericComputed('EnchantmentPassiveNum');

// Advanced Status & Corruptions
const twiceCorrupted = createBooleanComputed('TwiceCorrupted');
const scourged = createNumericComputed('Scourged');
const foulborn = createBooleanComputed('Foulborn');
const hasCruciblePassiveTree = createBooleanComputed('HasCruciblePassiveTree');
const hasVaalUniqueMod = createBooleanComputed('HasVaalUniqueMod');
const isVaalUnique = createBooleanComputed('IsVaalUnique');
const archnemesisMod = computed({
    get: () => getLineValue('ArchnemesisMod'),
    set: (v) => setLineValue('ArchnemesisMod', v, true)
});

// PoE 2 / Future
const waystoneTier = createNumericComputed('WaystoneTier');
const unidentifiedItemTier = createNumericComputed('UnidentifiedItemTier');

// Colors in 0-255 format
const textColor = computed({ 
    get: () => getLineValue('SetTextColor'), 
    set: (v) => setLineValue('SetTextColor', v) 
});
const bgColor = computed({ 
    get: () => getLineValue('SetBackgroundColor'), 
    set: (v) => setLineValue('SetBackgroundColor', v) 
});
const borderColor = computed({ 
    get: () => getLineValue('SetBorderColor'), 
    set: (v) => setLineValue('SetBorderColor', v) 
});

const validEffectColors = ['Red', 'Green', 'Blue', 'Brown', 'White', 'Yellow', 'Cyan', 'Grey', 'Orange', 'Pink', 'Purple'];

const effectColorMap: Record<string, string> = {
    'Red': '#ff4d4f',
    'Green': '#95de64',
    'Blue': '#69c0ff',
    'Brown': '#d4b106',
    'White': '#ffffff',
    'Yellow': '#fadb14',
    'Cyan': '#5cdbd3',
    'Grey': '#bfbfbf',
    'Orange': '#ffc069',
    'Pink': '#ff85c0',
    'Purple': '#b37feb'
};

const playEffectColor = computed({ 
    get: () => {
        const val = getLineValue('PlayEffect') || '';
        const parts = val.split(' ');
        // Check for any valid color in the string
        return validEffectColors.find(c => parts.includes(c)) || '';
    }, 
    set: (v) => {
        const current = getLineValue('PlayEffect') || '';
        const isTemp = current.includes('Temp');
        if (!v) {
            // If color is cleared, remove the entries line
            setLineValue('PlayEffect', '');
        } else {
            setLineValue('PlayEffect', `${v}${isTemp ? ' Temp' : ''}`);
        }
    } 
});

const playEffectTemp = computed({
    get: () => {
        const val = getLineValue('PlayEffect') || '';
        return val.includes('Temp');
    },
    set: (isTemp) => {
        const current = getLineValue('PlayEffect') || '';
        const parts = current.split(' ');
        const color = validEffectColors.find(c => parts.includes(c));
        
        if (color) {
             setLineValue('PlayEffect', `${color}${isTemp ? ' Temp' : ''}`);
        } else if (isTemp) {
            // User checked Temp but no color selected. Default to White?
            // Or just set "White Temp"?
            setLineValue('PlayEffect', `White Temp`);
        }
    }
});

const fontSize = computed({
    get: () => getLineValue('SetFontSize'), 
    set: (v) => {
        if (!v) {
            setLineValue('SetFontSize', '');
            return;
        }
        const n = parseInt(v);
        if (!isNaN(n)) {
            const clamped = Math.max(1, Math.min(45, n));
            setLineValue('SetFontSize', clamped.toString());
        }
    }
});
const minimapIcon = computed({
    get: () => getLineValue('MinimapIcon'), 
    set: (v) => setLineValue('MinimapIcon', v) 
});


// Helper to convert PoE color (space separated) to CSS hex for preview (approximate)
const toCssColor = (str: string) => {
    if (!str) return 'transparent';
    const parts = str.split(' ').map(n => parseInt(n));
    if (parts.length >= 3) {
        const a = parts[3] !== undefined ? parts[3] / 255 : 1;
        return `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, ${a})`;
    }
    return 'transparent';
};

const rgbStringToHex = (str: string): string => {
    if (!str) return '#000000';
    const parts = str.split(' ').map(n => parseInt(n));
    // Default to black if invalid
    if (parts.length < 3) return '#000000';
    
    const toHex = (n: number) => {
        const hex = Math.max(0, Math.min(255, isNaN(n) ? 0 : n)).toString(16);
        return hex.length === 1 ? '0' + hex : hex;
    };
    return `#${toHex(parts[0])}${toHex(parts[1])}${toHex(parts[2])}`;
};

const updateColorFromHex = (key: string, hex: string) => {
    // hex is #RRGGBB
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    
    // Get existing value to preserve alpha
    const currentStr = getLineValue(key);
    let alphaPart = '';
    if (currentStr) {
        const parts = currentStr.trim().split(/\s+/);
        if (parts.length >= 4) {
             alphaPart = ' ' + parts[3];
        }
    }
    
    setLineValue(key, `${r} ${g} ${b}${alphaPart}`);
};

const toggleExpand = () => {
    isExpanded.value = !isExpanded.value;
    if (isExpanded.value) {
        // Notify parent that this block was expanded/focused
        emit('focus', localBlock.value.id);
    }
};

// Known Keys to exclude from Custom Rules list
const knownKeys = new Set([
  'Class', 'BaseType', 
  'ItemLevel', 'DropLevel', 'Quality', 'Rarity', 'StackSize', 'Width', 'Height', 'Identified',
  'Sockets', 'LinkedSockets', 'SocketGroup',
  'BaseArmour', 'BaseEvasion', 'BaseEnergyShield', 'BaseWard', 'BaseDefencePercentile',
  'HasEaterOfWorldsImplicit', 'HasSearingExarchImplicit', 'HasEnchantment', 'AnyEnchantment',
  'EnchantmentPassiveNode', 'EnchantmentPassiveNum',
  'HasImplicitMod', 'Mirrored', 'FracturedItem', 'SynthesisedItem', 'Replica',
  'TwiceCorrupted', 'HasCruciblePassiveTree', 'Foulborn', 
  'ShaperItem', 'ElderItem', 'Corrupted', 'HasInfluence', 'HasExplicitMod', 'CorruptedMods',
  'MapTier', 'AreaLevel', 'GemLevel', 'TransfiguredGem',
  'BlightedMap', 'UberBlightedMap', 'ShapedMap', 'ElderMap',
  'WaystoneTier', 'UnidentifiedItemTier', 'ArchnemesisMod', 'Scourged', 'HasVaalUniqueMod', 'IsVaalUnique',
  'SetTextColor', 'SetBackgroundColor', 'SetBorderColor', 'PlayEffect', 'SetFontSize', 'MinimapIcon', 'PlayAlertSound', 'CustomAlertSound',
  'DisableDropSound', 'DisableDropSoundIfAlertSound', 'Continue'
]);

// Helper for Custom Rules
const addCustomRule = () => {
    localBlock.value.lines.push({
        key: 'NewRule',
        values: ['Value'], // default placeholder
        operator: undefined,
        raw: ''
    });
};

const removeLineAtIndex = (idx: number) => {
    localBlock.value.lines.splice(idx, 1);
};

</script>

<template>
  <div class="filter-block-card" :class="{ expanded: isExpanded }">
    <!-- Header Summary -->
    <div class="block-header" @click="toggleExpand" @contextmenu.prevent="onContextMenu">
      <div class="header-left">
        <div class="status-indicator" :class="localBlock.type.toLowerCase()"></div>
        <div class="header-info">
            <span class="header-title" :title="baseTypes || itemClass || localBlock.name">
                {{ baseTypes || itemClass || localBlock.name || localBlock.rawHeader || 'Untitled Rule' }}
            </span>
            <div class="preview-tags" v-if="!isExpanded">
                {{ [itemLevel ? 'Lvl ' + itemLevel : '', rarity].filter(Boolean).join(', ') }}
            </div>
        </div>
      </div>
      <div class="header-right">
         <select v-model="localBlock.type" class="glass-select small" @click.stop>
            <option value="Show">Show</option>
            <option value="Hide">Hide</option>
            <option value="Minimal">Minimal</option>
         </select>
         <div class="expand-icon">{{ isExpanded ? '▼' : '▶' }}</div>
      </div>
    </div>

    <!-- Expanded Editor -->
    <div v-if="isExpanded" class="block-body">
         
         <!-- Core Conditions (Most Important) -->
         <div class="form-row full-width">
            <label>Base Type (Item Name)</label>
            <textarea 
                ref="baseTypeInput"
                v-model.lazy="baseTypes" 
                class="glass-textarea" 
                rows="1"
                placeholder='e.g. "Divine Orb", "Chaos Orb"'
                @input="adjustTextareaHeight"
                @focus="adjustTextareaHeight"
             ></textarea>
         </div>

         <div class="form-row full-width">
            <label>Item Class</label>
            <input v-model.lazy="itemClass" class="glass-input" placeholder='e.g. "Currency" "Stackable Currency"' />
         </div>

         <!-- Detailed Conditions -->
         <div class="conditions-container">
            <!-- 1. Requirements & General -->
            <div class="section-title">General Requirements</div>
            <div class="form-grid four-col">
                <div class="form-group">
                    <label title="ItemLevel e.g. >= 85">Item Level</label>
                    <input v-model="itemLevel" class="glass-input small" placeholder=">= 60" />
                </div>
                <div class="form-group">
                    <label>Drop Level</label>
                    <input v-model="dropLevel" class="glass-input small" placeholder=">= 1" />
                </div>
                <div class="form-group">
                    <label>Quality</label>
                    <input v-model="quality" class="glass-input small" placeholder=">= 20" />
                </div>
                <div class="form-group">
                    <label>Rarity</label>
                    <select v-model="rarity" class="glass-select small">
                        <option value="">Any</option>
                        <option value="Normal">Normal</option>
                        <option value="Magic">Magic</option>
                        <option value="Rare">Rare</option>
                        <option value="Unique">Unique</option>
                        <option value=">= Normal">>= Normal</option>
                        <option value=">= Magic">>= Magic</option>
                        <option value=">= Rare">>= Rare</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Stack Size</label>
                    <input v-model="stackSize" class="glass-input small" placeholder=">= 10" />
                </div>
                <div class="form-group">
                    <label>Width</label>
                    <input v-model="width" class="glass-input small" placeholder="<= 1" />
                </div>
                <div class="form-group">
                    <label>Height</label>
                    <input v-model="height" class="glass-input small" placeholder="<= 2" />
                </div>
                 <div class="form-group">
                    <label>Identified</label>
                    <select v-model="identified" class="glass-select small">
                        <option value="">Ignore</option>
                        <option value="True">True</option>
                        <option value="False">False</option>
                    </select>
                </div>
            </div>

            <!-- 2. Sockets & Links -->
            <div class="section-title">Sockets</div>
            <div class="form-grid four-col">
                 <div class="form-group">
                     <label>Sockets (Count)</label>
                     <input v-model="sockets" class="glass-input small" placeholder="e.g. 6 or >= 5" />
                 </div>
                 <div class="form-group">
                     <label>Linked Sockets</label>
                     <input v-model="linkedSockets" class="glass-input small" placeholder="e.g. 6 or >= 5" />
                 </div>
                 <div class="form-group start-col-span-2">
                     <label>Socket Group (Specific Colors)</label>
                     <input v-model="socketGroup" class="glass-input small" placeholder="e.g. 3G or 4RGB or 6W" />
                 </div>
            </div>

            <!-- 3. Base Defences -->
            <div class="section-title">Base Stats (Defences)</div>
            <div class="form-grid four-col">
                 <div class="form-group">
                     <label>Armour</label>
                     <input v-model="baseArmour" class="glass-input small" placeholder="> 0" />
                 </div>
                 <div class="form-group">
                     <label>Evasion</label>
                     <input v-model="baseEvasion" class="glass-input small" placeholder="> 0" />
                 </div>
                 <div class="form-group">
                     <label>Energy Shield</label>
                     <input v-model="baseES" class="glass-input small" placeholder="> 0" />
                 </div>
                 <div class="form-group">
                     <label>Base Percentile</label>
                     <!-- using baseDefencePct as defined in script -->
                     <input v-model="baseDefencePct" class="glass-input small" placeholder=">= 90" />
                 </div>
                 <div class="form-group">
                     <label>Ward</label>
                     <input v-model="baseWard" class="glass-input small" placeholder="> 0" />
                 </div>
            </div>

            <!-- 4. Influence, Enchant & Status -->
            <div class="section-title">Enchants, Clusters & Eldritch</div>
            <div class="form-grid four-col">
                <div class="form-group">
                    <label>Eater Tier (1-6)</label>
                    <input v-model="hasEaterOfWorldsImplicit" class="glass-input small" placeholder=">= 1" />
                </div>
                <div class="form-group">
                    <label>Exarch Tier (1-6)</label>
                    <input v-model="hasSearingExarchImplicit" class="glass-input small" placeholder=">= 1" />
                </div>
                 <div class="form-group start-col-span-2">
                    <label>Enchant Name</label>
                    <input v-model.lazy="hasEnchantment" class="glass-input small" placeholder='Name / "Tier"' />
                </div>
                
                <!-- Cluster Jewels -->
                <div class="form-group start-col-span-2">
                    <label>Cluster Passive (Small Passives)</label>
                    <input v-model.lazy="enchantmentPassiveNode" class="glass-input small" placeholder='"Mace Damage" etc' />
                </div>
                <div class="form-group">
                    <label>Cluster Count</label>
                    <input v-model="enchantmentPassiveNum" class="glass-input small" placeholder="<= 5" />
                </div>
                
                <!-- Boolean Toggles Row -->
                <div class="form-group checkbox-group-inline full-width">
                    <label class="bool-check" title="Any Enchantment"><input type="checkbox" v-model="anyEnchantment" true-value="True" false-value="" /> Enchanted</label>
                    <label class="bool-check" title="Has Implicit"><input type="checkbox" v-model="hasImplicitMod" true-value="True" false-value="" /> Implicit</label>
                    <label class="bool-check"><input type="checkbox" v-model="mirrored" true-value="True" false-value="" /> Mirrored</label>
                    <label class="bool-check"><input type="checkbox" v-model="fracturedItem" true-value="True" false-value="" /> Fractured</label>
                    <label class="bool-check"><input type="checkbox" v-model="synthesisedItem" true-value="True" false-value="" /> Synth</label>
                    <label class="bool-check"><input type="checkbox" v-model="replica" true-value="True" false-value="" /> Replica</label>
                    <label class="bool-check"><input type="checkbox" v-model="twiceCorrupted" true-value="True" false-value="" /> Dbl Corrupt</label>
                    <label class="bool-check"><input type="checkbox" v-model="hasCruciblePassiveTree" true-value="True" false-value="" /> Crucible</label>
                    <label class="bool-check"><input type="checkbox" v-model="foulborn" true-value="True" false-value="" /> Foulborn</label>
                </div>
            </div>

            <div class="form-grid four-col">
                <div class="form-group checkbox-group-inline start-col-span-2">
                     <label class="bool-check"><input type="checkbox" v-model="shaperItem" true-value="True" false-value="" /> Shaper Item</label>
                     <label class="bool-check"><input type="checkbox" v-model="elderItem" true-value="True" false-value="" /> Elder Item</label>
                </div>
                <div class="form-group">
                    <label>Corrupted</label>
                    <select v-model="corrupted" class="glass-select small">
                        <option value="">Ignore</option>
                        <option value="True">Yes</option>
                        <option value="False">No</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Influence</label>
                    <select v-model="hasInfluence" class="glass-select small">
                        <option value="">Ignore</option>
                        <option value="Shaper">Shaper</option>
                        <option value="Elder">Elder</option>
                        <option value="Crusader">Crusader</option>
                        <option value="Hunter">Hunter</option>
                        <option value="Redeemer">Redeemer</option>
                        <option value="Warlord">Warlord</option>
                        <option value="None">None</option>
                    </select>
                </div>
                 <div class="form-group">
                     <label>Explicit Mods</label>
                     <input v-model.lazy="hasExplicitMod" class="glass-input small" placeholder='>= 1 "Mod Name"' />
                 </div>
                 <div class="form-group">
                     <label>Corrupted Mods</label>
                     <input v-model="corruptedMods" class="glass-input small" placeholder=">= 1" />
                 </div>
            </div>

            <!-- 5. Map & Gems -->
            <div class="section-title">Maps & Gems</div>
            <div class="form-grid four-col">
                 <div class="form-group">
                     <label>Map Tier</label>
                     <input v-model="mapTier" class="glass-input small" placeholder=">= 1" />
                 </div>
                 <div class="form-group">
                     <label>Area Level</label>
                     <input v-model="areaLevel" class="glass-input small" placeholder=">= 68" />
                 </div>
                 <div class="form-group">
                     <label>Gem Level</label>
                     <input v-model="gemLevel" class="glass-input small" placeholder=">= 20" />
                 </div>
                  <div class="form-group">
                     <label>Transfigured</label>
                     <input v-model.lazy="transfiguredGem" class="glass-input small" placeholder="True / Name" />
                 </div>
            </div>
            <div class="form-row checkbox-row">
                 <label class="bool-check"><input type="checkbox" v-model="blightedMap" true-value="True" false-value="" /> Blighted Map</label>
                 <label class="bool-check"><input type="checkbox" v-model="uberBlightedMap" true-value="True" false-value="" /> Uber Blight</label>
                 <label class="bool-check"><input type="checkbox" v-model="shapedMap" true-value="True" false-value="" /> Shaped</label>
                 <label class="bool-check"><input type="checkbox" v-model="elderMap" true-value="True" false-value="" /> Elder Map</label>
            </div>
            
            <!-- 6. Special / PoE 2 -->
             <div class="section-title">Special & Future (PoE 2)</div>
            <div class="form-grid four-col">
                <div class="form-group">
                    <label>Waystone Tier</label>
                    <input v-model="waystoneTier" class="glass-input small" placeholder=">= 1" />
                </div>
                 <div class="form-group">
                    <label>Unidentified Tier</label>
                    <input v-model="unidentifiedItemTier" class="glass-input small" placeholder=">= 1" />
                </div>
                 <div class="form-group">
                    <label>Archnemesis Mod</label>
                    <input v-model.lazy="archnemesisMod" class="glass-input small" placeholder='Name' />
                </div>
                 <div class="form-group">
                    <label>Scourge Tier</label>
                    <input v-model="scourged" class="glass-input small" placeholder=">= 1" />
                </div>
                <!-- Boolean Toggles Row -->
                <div class="form-group checkbox-group-inline full-width">
                     <label class="bool-check"><input type="checkbox" v-model="hasVaalUniqueMod" true-value="True" false-value="" /> Vaal Unique Mod</label>
                     <label class="bool-check"><input type="checkbox" v-model="isVaalUnique" true-value="True" false-value="" /> Is Vaal Unique</label>
                </div>
            </div>
            
            <!-- 7. Custom / Other Rules -->
            <div class="section-title">Other Rules (Raw Edit)</div>
            <div class="custom-rules-list">
                <div v-for="(line, idx) in localBlock.lines" :key="idx">
                    <div v-if="!knownKeys.has(line.key)" class="custom-rule-row">
                         <input v-model="line.key" class="glass-input small key-input" placeholder="Key" />
                         <select v-model="line.operator" class="glass-select small op-select">
                            <option :value="undefined"></option>
                            <option value="=">=</option>
                            <option value="==">==</option>
                            <option value=">">&gt;</option>
                            <option value="<">&lt;</option>
                            <option value=">=">&gt;=</option>
                            <option value="<=">&lt;=</option>
                         </select>
                         <input 
                            :value="line.values.join(' ')" 
                            @input="(e: Event) => line.values = (e.target as HTMLInputElement).value.split(' ')"
                            class="glass-input small value-input" 
                            placeholder="Value(s)" 
                         />
                         <button @click="removeLineAtIndex(idx)" class="glass-button icon danger">🗑️</button>
                    </div>
                </div>
                <button @click="addCustomRule" class="glass-button small full-width dashed">+ Add Custom Rule</button>
            </div>
         </div>

         <!-- Metadata / Comments -->
         <div class="form-grid">
             <div class="form-group start-col-span-3">
                 <label>Comment Header / Description</label>
                 <textarea v-model="localBlock.rawHeader" class="glass-textarea small" rows="2" placeholder="# Comments..."></textarea>
             </div>
         </div>
         
         <!-- Classification (Comments) -->
         <div class="form-grid">
             <div class="form-group">
                 <label>Category (Tag)</label>
                 <input v-model="localBlock.category" class="glass-input small" placeholder="e.g. 基础" />
             </div>
             <div class="form-group">
                 <label>Alias / Custom Name</label>
                 <input v-model="localBlock.name" class="glass-input small" placeholder="e.g. 货币通货" />
             </div>
             <div class="form-group">
                 <label>Priority (Tag)</label>
                 <input v-model="localBlock.priority" class="glass-input small" placeholder="e.g. 1" />
             </div>
         </div>

         <!-- Colors -->
         <div class="form-grid">
            <div class="form-group">
                <label>文字颜色</label>
                <div class="color-input-group">
                    <div class="color-picker-wrapper">
                        <div class="color-preview" :style="{ background: toCssColor(textColor) }"></div>
                        <input type="color" :value="rgbStringToHex(textColor)" @input="e => updateColorFromHex('SetTextColor', (e.target as HTMLInputElement).value)" class="hidden-color-input">
                    </div>
                    <input v-model="textColor" class="glass-input small" placeholder="255 255 255" />
                </div>
            </div>
            <div class="form-group">
                 <label>背景颜色</label>
                 <div class="color-input-group">
                    <div class="color-picker-wrapper">
                        <div class="color-preview" :style="{ background: toCssColor(bgColor) }"></div>
                        <input type="color" :value="rgbStringToHex(bgColor)" @input="e => updateColorFromHex('SetBackgroundColor', (e.target as HTMLInputElement).value)" class="hidden-color-input">
                    </div>
                     <input v-model="bgColor" class="glass-input small" placeholder="0 0 0 240" />
                 </div>
             </div>
             <div class="form-group">
                 <label>边框颜色</label>
                 <div class="color-input-group">
                    <div class="color-picker-wrapper">
                        <div class="color-preview" :style="{ background: toCssColor(borderColor), borderColor: '#fff' }"></div>
                        <input type="color" :value="rgbStringToHex(borderColor)" @input="e => updateColorFromHex('SetBorderColor', (e.target as HTMLInputElement).value)" class="hidden-color-input">
                    </div>
                     <input v-model="borderColor" class="glass-input small" placeholder="255 0 0" />
                 </div>
             </div>
         </div>

         <!-- Display & Sound -->
         <div class="form-grid four-col">
             <div class="form-group">
                 <label>字体大小</label>
                 <input type="number" v-model.lazy="fontSize" min="1" max="45" class="glass-input small" placeholder="32" />
             </div>
             <div class="form-group">
                 <label>光柱颜色</label>
                 <div style="display: flex; gap: 8px; align-items: center;">
                    <select v-model="playEffectColor" class="glass-select small" :style="{ flex: 1, color: effectColorMap[playEffectColor] || 'inherit' }">
                        <option value="" style="color: #ccc; background-color: rgba(0,0,0,0.8);">None</option>
                        <option v-for="c in validEffectColors" :key="c" :value="c" :style="{ color: effectColorMap[c], backgroundColor: 'rgba(0,0,0,0.8)' }">{{ c }}</option>
                    </select>
                    <label class="bool-check" style="margin-bottom: 0;">
                        <input type="checkbox" v-model="playEffectTemp" />
                        只在掉落时显示光柱
                    </label>
                 </div>
             </div>
             <div class="form-group">
                 <label>Minimap Icon</label>
                 <input v-model="minimapIcon" class="glass-input small" placeholder="Size Clr Shape" />
             </div>
             <div class="form-group start-col-span-2">
                 <label>掉落音效</label>
                 <div style="display: flex; gap: 4px; align-items: center;">
                    <select v-model="alertSoundId" class="glass-select small" style="flex: 1;">
                        <option value="">None</option>
                        <option v-for="n in 16" :key="n" :value="n.toString()">Sound {{ n }}</option>
                    </select>
                    <div class="input-suffix-group" style="display: flex; align-items: center; position: relative; width: 80px;">
                        <input type="number" v-model="alertSoundVolume" min="0" max="300" class="glass-input small" style="width: 100%; padding-right: 25px; text-align: center;" title="Volume (0-300)" />
                        <span style="position: absolute; right: 5px; font-size: 10px; color: rgba(255,255,255,0.5); pointer-events: none;">音量</span>
                    </div>
                 </div>
             </div>
             <div class="form-group">
                 <label>Custom Sound</label>
                 <div class="input-group">
                    <input v-model="customAlertSound" class="glass-input small" placeholder='File "Vol"' />
                    <button @click="browseSound" class="glass-button icon" title="Select File">📂</button>
                 </div>
             </div>
         </div>
         
         <div class="form-row checkbox-row">
             <label class="checkbox-label">
                 <input type="checkbox" v-model="disableDropSound" />
                 <span>Disable Drop Sound</span>
             </label>
             <label class="checkbox-label">
                 <input type="checkbox" v-model="disableDropSoundIfAlertSound" />
                 <span>Quiet if Alert</span>
             </label>
             <label class="checkbox-label">
                 <input type="checkbox" v-model="shouldContinue" />
                 <span>Continue (Match Next)</span>
             </label>
         </div>
    </div>
  </div>
</template>

<style scoped>
.filter-block-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 6px;
  margin-bottom: 8px;
  overflow: hidden;
  transition: all 0.2s;
}

.filter-block-card:hover {
    background: rgba(255, 255, 255, 0.06);
}

.filter-block-card.expanded {
    background: rgba(20, 20, 20, 0.6);
    border-color: rgba(64, 158, 255, 0.3);
}

.block-header {
  padding: 8px 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
}

.header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
}

.status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #666;
}

.status-indicator.show { background: #67c23a; box-shadow: 0 0 4px #67c23a; }
.status-indicator.hide { background: #f56c6c; }

.header-info {
    flex: 1;
    display: flex;
    flex-direction: column;
}

.header-title {
    font-weight: bold;
    font-size: 14px;
    color: #eee;
}

.preview-tags {
    font-size: 11px;
    color: #888;
    margin-top: 2px;
}

.header-right {
    display: flex;
    align-items: center;
    gap: 8px;
}

.block-body {
    padding: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.form-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
}

.start-col-span-2 { grid-column: span 2; }
.start-col-span-3 { grid-column: span 3; }

.form-row.full-width {
    width: 100%;
}

label {
    display: block;
    font-size: 11px;
    color: #aaa;
    margin-bottom: 4px;
}

.color-input-group {
    display: flex;
    align-items: center;
    gap: 8px;
}

.color-preview {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    border: 1px solid #444;
}

.color-picker-wrapper {
    position: relative;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
}

.hidden-color-input {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
    border: none;
    padding: 0;
    margin: 0;
}

/* Glass Inputs */
.glass-input, .glass-textarea, .glass-select {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #eee;
    padding: 6px 8px;
    border-radius: 4px;
    font-family: inherit;
    width: 100%;
    box-sizing: border-box;
}

.glass-input:focus, .glass-textarea:focus, .glass-select:focus {
    outline: none;
    border-color: #409eff;
    background: rgba(0, 0, 0, 0.4);
}

.input-group {
    display: flex;
    gap: 4px;
}
.glass-button.icon {
    padding: 2px 6px;
    font-size: 14px;
    line-height: 1;
}

.glass-input-transparent {
    background: transparent;
    border: 1px solid transparent;
    color: inherit;
    padding: 0;
    width: 100%;
}
.glass-input-transparent:focus {
    outline: none;
    border-bottom: 1px solid #409eff;
}

.glass-input.small {
    padding: 4px 6px;
    font-size: 12px;
}

.glass-select.small {
    width: auto;
    padding: 2px 8px;
    font-size: 12px;
    height: 24px;
}
.checkbox-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
}
.form-grid.four-col {
    grid-template-columns: repeat(4, 1fr);
}
.section-title {
    font-size: 11px;
    text-transform: uppercase;
    color: #666;
    margin: 8px 0 4px 0;
    font-weight: bold;
    letter-spacing: 0.5px;
    border-bottom: 1px solid rgba(255,255,255,0.05);
    padding-bottom: 2px;
}
.checkbox-group-inline {
    display: flex;
    gap: 12px;
    align-items: center;
    height: 100%;
}
.bool-check {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 12px;
}
.conditions-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
    color: #eee;
}

.form-group.start-col-span-2 {
    grid-column: span 2;
}

.glass-textarea.small {
    font-size: 12px;
    padding: 6px;
    line-height: normal;
}

.custom-rules-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
}
.custom-rule-row {
    display: flex;
    gap: 8px;
    align-items: center;
}
.key-input {
    width: 140px !important;
    flex-shrink: 0;
}
.op-select {
    width: 60px !important;
    flex-shrink: 0;
}
.value-input {
    flex: 1;
}
.full-width.dashed {
    border-style: dashed;
    margin-top: 8px;
    opacity: 0.6;
}
.full-width.dashed:hover {
    opacity: 1;
    border-style: solid;
}
</style>
export interface ParsedMod {
  type: string;
  text: string;
  tier?: number;
}

export interface ParsedItem {
  raw: string;
  rarity: string;
  name: string;
  baseType: string;
  itemClass: string;
  itemLevel?: number;
  quality?: number;
  requirements: Record<string, number>;
  stats: Record<string, string>;
  mods: ParsedMod[];
  sockets?: { count: number; links: number };
}

// Normalize Chinese text by removing spaces between CJK characters + remove all spaces
// "稀 有 度: 稀有" → "稀有度:稀有" for label matching
function norm(s: string): string {
  return s.replace(/\s+/g, '');
}

// Check if normalized line starts with any of the given prefixes (also normalized)
function matchLabel(line: string, prefixes: string[]): boolean {
  const n = norm(line).toLowerCase();
  return prefixes.some(p => n.startsWith(norm(p).toLowerCase()));
}

// Extract value after label prefix
function extractAfter(line: string, prefixes: string[]): string | null {
  const n = norm(line);
  for (const p of prefixes) {
    const np = norm(p);
    const idx = n.indexOf(np);
    if (idx !== -1) {
      return n.substring(idx + np.length).replace(/^[:：]\s*/, '').trim();
    }
  }
  return null;
}

// Check if a line looks like flavor text (long Chinese sentence without game stat patterns)
function isFlavorText(line: string): boolean {
  // Flavor text: long, no numbers, no colons, no game terms
  if (line.length > 25 && !/\d/.test(line) && !line.includes(':')) {
    return true;
  }
  return false;
}

// Mod type detection from POE2 suffixes
function detectModType(text: string): string {
  const lower = text.toLowerCase();
  if (lower.includes('(implicit)') || lower.includes('(基底)')) return 'Implicit';
  if (lower.includes('(rune)') || lower.includes('(符文)')) return 'Rune';
  if (lower.includes('(crafted)') || lower.includes('(工艺)')) return 'Crafted';
  if (lower.includes('(enchant)') || lower.includes('(附魔)')) return 'Enchant';
  return 'Explicit';
}

function cleanModText(text: string): string {
  return text.replace(/\s*\((implicit|rune|crafted|enchant|augmented|基底|符文|工艺|附魔|增强)\)/gi, '').trim();
}

function extractTier(line: string): number | undefined {
  const m = line.match(/\((?:Tier|阶级|階級)\s*:\s*(\d+)\)/i);
  if (m) return parseInt(m[1]);
  const t = line.match(/\bT(\d+)\b/i);
  if (t) return parseInt(t[1]);
  return undefined;
}

// Stat-line: has a colon, value has numbers
function parseStatLine(line: string): { key: string; value: string } | null {
  const SKIP = new Set([
    'itemclass', '物品类别',
    'rarity', '稀有度',
    'itemlevel', '物品等级',
    'sockets', '孔数', '插槽',
    'requirements', '需求',
    'level', '等级', 'str', 'strength', '力量', 'dex', '敏捷', 'int', '智力',
  ]);

  // Clean [Tag|Label] brackets
  const cleaned = line.replace(/\[.*?\|(.*?)\]/g, '$1').replace(/\[(.*?)\]/g, '$1');
  const m = cleaned.match(/^(.+?)[:：]\s*(.+)$/);
  if (!m) return null;

  const key = m[1].trim();
  const value = m[2].trim();

  if (SKIP.has(norm(key).toLowerCase())) return null;

  return { key, value };
}

// Parse requirement from "[Strength|Str]: 36" or "等级: 33" or "力量: 36"
const REQ_MAP: Record<string, string> = {
  'level': 'level', '等级': 'level',
  'str': 'str', 'strength': 'str', '力量': 'str',
  'dex': 'dex', 'dexterity': 'dex', '敏捷': 'dex',
  'int': 'int', 'intelligence': 'int', '智力': 'int',
};

function parseRequirement(line: string): { key: string; value: number } | null {
  const cleaned = line.replace(/\[.*?\|(.*?)\]/g, '$1').replace(/\[(.*?)\]/g, '$1');
  const m = cleaned.match(/^(.+?)[:：]\s*(\d+)/);
  if (!m) return null;
  const keyPart = norm(m[1]).toLowerCase();
  const val = parseInt(m[2]);
  for (const [label, key] of Object.entries(REQ_MAP)) {
    if (keyPart === norm(label) || keyPart.includes(norm(label))) {
      return { key, value: val };
    }
  }
  return null;
}

// ============ Main entry ============

export function parseItemText(text: string): ParsedItem | null {
  if (!text || text.trim().length === 0) return null;

  const rawLines = text.split('\n');
  const lines = rawLines.map(l => l.trim()).filter(l => l.length > 0);
  if (lines.length < 3) return null;

  // Detect format:
  // POE2: first line is "Item Class:" / "物品类别:"
  // POE1: first line is "Rarity:" / "稀有度:" with { } brace mods later
  const isPoe2 = matchLabel(lines[0], ['item class:', '物品类别:']);
  const hasBraceMods = lines.some(l => /^\{\s*\w+:\s*"/.test(l));

  if (!isPoe2 && !hasBraceMods) {
    // Neither clear marker — check if has rarity label
    if (!lines.some(l => matchLabel(l, ['rarity:', '稀有度:']))) {
      return null;
    }
    // Try POE2 first since user is on POE2
    return parsePoe2Format(lines);
  }

  if (isPoe2 || !hasBraceMods) {
    return parsePoe2Format(lines);
  }
  return parsePoe1Format(lines);
}

// ============ POE2 Parser ============

function parsePoe2Format(lines: string[]): ParsedItem | null {
  const item: ParsedItem = {
    raw: lines.join('\n'),
    rarity: '',
    name: '',
    baseType: '',
    itemClass: '',
    requirements: {},
    stats: {},
    mods: [],
  };

  let i = 0;

  // Line 1: Item Class
  if (i < lines.length && matchLabel(lines[i], ['item class:', '物品类别:'])) {
    item.itemClass = extractAfter(lines[i], ['Item Class:', '物品类别:']) || '';
    i++;
  }

  // Line 2: Rarity (may have spaces: "稀 有 度")
  if (i < lines.length && matchLabel(lines[i], ['rarity:', '稀有度:', '稀 有 度:'])) {
    item.rarity = extractAfter(lines[i], ['Rarity:', '稀有度:', '稀 有 度:']) || '';
    i++;
  }

  // Lines after rarity until separator or labeled line: item name + base type
  const nameLines: string[] = [];
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith('--------')) break;
    // Stop at labeled lines
    if (norm(line).match(/^[a-z一-鿿]+[:：]/) && nameLines.length > 0) break;
    nameLines.push(line);
    i++;
  }

  if (nameLines.length >= 1) {
    item.name = nameLines[0];
    item.baseType = nameLines[nameLines.length - 1];
  }
  if (nameLines.length === 1) {
    item.baseType = item.name;
  }

  // Skip separator
  if (i < lines.length && lines[i].startsWith('--------')) i++;

  // Parse body: iterate through separator-delimited sections
  let inRequirements = false;
  let pastItemLevel = false;

  while (i < lines.length) {
    const line = lines[i];

    // Separator
    if (line.startsWith('--------')) {
      i++;
      inRequirements = false;
      continue;
    }

    // Item Level
    if (matchLabel(line, ['item level:', '物品等级:'])) {
      const val = extractAfter(line, ['Item Level:', '物品等级:']);
      if (val) {
        const ilvl = parseInt(val);
        if (!isNaN(ilvl)) {
          item.itemLevel = ilvl;
          pastItemLevel = true;
        }
      }
      i++;
      continue;
    }

    // Requirements header
    if (matchLabel(line, ['requirements:', '需求:'])) {
      inRequirements = true;
      i++;
      continue;
    }

    // Requirement lines
    if (inRequirements) {
      const req = parseRequirement(line);
      if (req) {
        item.requirements[req.key] = req.value;
        i++;
        continue;
      }
      inRequirements = false;
    }

    // Quality (stat line that starts with quality)
    if (matchLabel(line, ['quality:', '品质:'])) {
      const qm = norm(line).match(/([+-]?\d+)%/);
      if (qm) item.quality = parseInt(qm[1]);
      i++;
      continue;
    }

    // Sockets
    if (matchLabel(line, ['sockets:', '孔数:', '插槽:'])) {
      const val = extractAfter(line, ['Sockets:', '孔数:', '插槽:']);
      if (val) {
        const parts = val.trim().split(/\s+/);
        let count = 0, maxLinks = 0;
        for (const part of parts) {
          if (part.includes('-')) {
            const links = part.split('-');
            count += links.length;
            if (links.length > maxLinks) maxLinks = links.length;
          } else {
            count++;
            if (1 > maxLinks) maxLinks = 1;
          }
        }
        item.sockets = { count, links: maxLinks };
      }
      i++;
      continue;
    }

    // Labeled stat line (has colon, key doesn't match special labels)
    const stat = parseStatLine(line);
    if (stat) {
      item.stats[stat.key] = stat.value;
      i++;
      continue;
    }

    // Skip flavor text (long sentences without stats after last separator)
    if (pastItemLevel && isFlavorText(line)) {
      i++;
      continue;
    }

    // Everything else that's not a separator or labeled line = mod
    // Skip empty lines and obvious non-mod lines
    if (line.length > 0 && !line.startsWith('--------') && !line.startsWith('{')) {
      const modType = detectModType(line);
      const tier = extractTier(line);
      const cleanText = cleanModText(line);
      if (cleanText.length > 0 && cleanText.length < 200) {
        item.mods.push({ type: modType, text: cleanText, tier });
      }
    }
    i++;
  }

  if (!item.rarity) item.rarity = 'Rare';
  if (!item.baseType) item.baseType = item.name;
  return item;
}

// ============ POE1 Parser (brace-mod format) ============

function parsePoe1Format(lines: string[]): ParsedItem | null {
  const item: ParsedItem = {
    raw: lines.join('\n'),
    rarity: '',
    name: '',
    baseType: '',
    itemClass: '',
    requirements: {},
    stats: {},
    mods: [],
  };

  let i = 0;

  // Rarity
  if (matchLabel(lines[i], ['rarity:', '稀有度:'])) {
    item.rarity = extractAfter(lines[i], ['Rarity:', '稀有度:']) || '';
    i++;
  }

  // Name + base type
  const nameLines: string[] = [];
  while (i < lines.length && !lines[i].startsWith('--------')) {
    nameLines.push(lines[i]);
    i++;
  }
  if (nameLines.length > 0) item.name = nameLines[0];
  if (nameLines.length > 1) item.baseType = nameLines[nameLines.length - 1];
  else item.baseType = item.name;

  // Class section
  if (lines[i]?.startsWith('--------')) i++;
  const classLines: string[] = [];
  while (i < lines.length && !lines[i].startsWith('--------')) {
    classLines.push(lines[i]);
    i++;
  }
  if (classLines.length > 0) item.itemClass = classLines[0];

  if (lines[i]?.startsWith('--------')) i++;

  // Stats
  while (i < lines.length && !lines[i].startsWith('--------')) {
    if (matchLabel(lines[i], ['quality:', '品质:'])) {
      const qm = norm(lines[i]).match(/([+-]?\d+)%/);
      if (qm) item.quality = parseInt(qm[1]);
    }
    if (matchLabel(lines[i], ['sockets:', '孔数:'])) {
      const val = extractAfter(lines[i], ['Sockets:', '孔数:']);
      if (val) {
        const parts = val.trim().split(/\s+/);
        let count = 0, maxLinks = 0;
        for (const part of parts) {
          if (part.includes('-')) {
            const links = part.split('-');
            count += links.length;
            if (links.length > maxLinks) maxLinks = links.length;
          } else { count++; if (1 > maxLinks) maxLinks = 1; }
        }
        item.sockets = { count, links: maxLinks };
      }
    }
    const st = parseStatLine(lines[i]);
    if (st) item.stats[st.key] = st.value;
    i++;
  }

  if (lines[i]?.startsWith('--------')) i++;

  // Requirements
  while (i < lines.length && !lines[i].startsWith('--------') &&
         !matchLabel(lines[i], ['item level:', '物品等级:'])) {
    const req = parseRequirement(lines[i]);
    if (req) item.requirements[req.key] = req.value;
    i++;
  }

  // Remaining: item level, mods
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith('--------')) { i++; continue; }

    if (matchLabel(line, ['item level:', '物品等级:'])) {
      const val = extractAfter(line, ['Item Level:', '物品等级:']);
      if (val) { const ilvl = parseInt(val); if (!isNaN(ilvl)) item.itemLevel = ilvl; }
      i++; continue;
    }

    // POE1 brace mod
    const bm = line.match(/^\{\s*(\w+):\s*"([^"]*)"\s+(.+?)\s*(?:\((?:Tier|阶级|階級):\s*(\d+)\))?\s*\}$/i);
    if (bm) {
      item.mods.push({
        type: bm[1],
        text: bm[3].trim(),
        tier: bm[4] ? parseInt(bm[4]) : undefined,
      });
      i++; continue;
    }

    i++;
  }

  return item;
}

export function generateTradeSearchName(item: ParsedItem): string {
  if (item.rarity === 'Unique' || item.rarity === '传奇') {
    return item.name;
  }
  return item.baseType || item.name;
}

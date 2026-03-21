import type { ProjectEntry } from "$lib/tauri/api";

export type FileTreeNode = {
  name: string;
  path: string;
  isDir: boolean;
  children: FileTreeNode[];
};

type Wip = {
  name: string;
  path: string;
  isDir: boolean;
  children: Map<string, Wip>;
};

function toSortedNodes(map: Map<string, Wip>): FileTreeNode[] {
  const nodes = [...map.values()];
  nodes.sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
  return nodes.map((n) => ({
    name: n.name,
    path: n.path,
    isDir: n.isDir,
    children: toSortedNodes(n.children),
  }));
}

/** Build a nested tree from flat project entries (paths use `/`). */
export function buildFileTree(entries: readonly ProjectEntry[]): FileTreeNode[] {
  const root = new Map<string, Wip>();

  function ensureDirChain(parts: string[]): Wip {
    let level = root;
    let acc: string[] = [];
    let last: Wip | undefined;
    for (const seg of parts) {
      acc.push(seg);
      const fullPath = acc.join("/");
      let n = level.get(seg);
      if (!n) {
        n = { name: seg, path: fullPath, isDir: true, children: new Map() };
        level.set(seg, n);
      }
      last = n;
      level = n.children;
    }
    return last!;
  }

  for (const e of entries) {
    const parts = e.path.split("/").filter(Boolean);
    if (parts.length === 0) continue;

    if (e.isDir) {
      ensureDirChain(parts);
      continue;
    }

    const parentParts = parts.slice(0, -1);
    const name = parts[parts.length - 1]!;
    const parent = parentParts.length ? ensureDirChain(parentParts) : null;
    const level = parent?.children ?? root;
    const filePath = parts.join("/");
    if (!level.has(name)) {
      level.set(name, {
        name,
        path: filePath,
        isDir: false,
        children: new Map(),
      });
    } else {
      const existing = level.get(name)!;
      existing.isDir = false;
    }
  }

  return toSortedNodes(root);
}

import { SideNav } from "@astryxdesign/core/SideNav";
import { Text } from "@astryxdesign/core/Text";
import { TreeList, type TreeListItemData } from "@astryxdesign/core/TreeList";
import { useMemo } from "react";
import type { PlaylistNode } from "../../library/api";
import { ALL_TRACKS_ID } from "../../library/useLibrary";

interface Props {
  tree: PlaylistNode[];
  selectedId: string;
  onSelect: (id: string) => void;
  isSyncing: boolean;
}

function toItems(
  nodes: PlaylistNode[],
  selectedId: string,
  onSelect: (id: string) => void,
): TreeListItemData[] {
  return nodes.map((node) =>
    node.kind === "folder"
      ? {
          id: node.id,
          label: node.name,
          // A folder row toggles itself when it has no onClick.
          children: toItems(node.children, selectedId, onSelect),
        }
      : {
          id: node.id,
          label: node.name,
          description: node.kind === "smart" ? "Smart playlist" : undefined,
          isSelected: node.id === selectedId,
          onClick: () => onSelect(node.id),
        },
  );
}

export function PlaylistSidebar({
  tree,
  selectedId,
  onSelect,
  isSyncing,
}: Props) {
  const items = useMemo<TreeListItemData[]>(
    () => [
      {
        id: ALL_TRACKS_ID,
        label: "All Tracks",
        isSelected: selectedId === ALL_TRACKS_ID,
        onClick: () => onSelect(ALL_TRACKS_ID),
      },
      ...toItems(tree, selectedId, onSelect),
    ],
    [tree, selectedId, onSelect],
  );

  return (
    <SideNav
      aria-label="Playlists"
      resizable={{
        defaultWidth: 260,
        minWidth: 180,
        maxWidth: 480,
        autoSaveId: "syncbox.sidebar",
      }}
    >
      <div className="sb-sidebar">
        <TreeList items={items} density="compact" />
        {tree.length === 0 && (
          <Text type="supporting" className="sb-sidebar-hint">
            {isSyncing
              ? "Syncing your Rekordbox Library…"
              : "No Playlists yet. Sync to bring in your Rekordbox Library."}
          </Text>
        )}
      </div>
    </SideNav>
  );
}

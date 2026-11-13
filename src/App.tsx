import { AppShell } from "@astryxdesign/core/AppShell";
import { Banner } from "@astryxdesign/core/Banner";
import { Button } from "@astryxdesign/core/Button";
import { Heading } from "@astryxdesign/core/Heading";
import { ProgressBar } from "@astryxdesign/core/ProgressBar";
import { Text } from "@astryxdesign/core/Text";
import { TextInput } from "@astryxdesign/core/TextInput";
import { TopNav } from "@astryxdesign/core/TopNav";
import { Theme } from "@astryxdesign/core/theme";
import { gothicTheme } from "@astryxdesign/theme-gothic/built";
import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { PlaylistSidebar } from "./features/sidebar/PlaylistSidebar";
import { TrackTable } from "./features/track-table/TrackTable";
import type { PlaylistNode } from "./library/api";
import { overallProgress, progressLabel } from "./library/syncProgress";
import { filterRows, toTableRows } from "./library/trackRows";
import { ALL_TRACKS_ID, useLibrary } from "./library/useLibrary";
import "./App.css";

function findName(nodes: PlaylistNode[], id: string): string | null {
  for (const n of nodes) {
    if (n.id === id) return n.name;
    const inner = findName(n.children, id);
    if (inner) return inner;
  }
  return null;
}

export default function App() {
  const library = useLibrary();
  const { sync, selectedId, tree } = library;
  const isSyncing = sync.status === "running";

  const [filter, setFilter] = useState("");
  const deferredFilter = useDeferredValue(filter);
  // A new list starts unfiltered.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on selection change only.
  useEffect(() => setFilter(""), [selectedId]);

  const allRows = useMemo(() => toTableRows(library.tracks), [library.tracks]);
  const rows = useMemo(
    () => filterRows(allRows, deferredFilter),
    [allRows, deferredFilter],
  );

  const listName =
    selectedId === ALL_TRACKS_ID
      ? "All Tracks"
      : (findName(tree, selectedId) ?? "Playlist");
  const count = new Intl.NumberFormat().format(rows.length);
  const isFiltered = deferredFilter.trim() !== "";

  const topNav = (
    <div className="sb-top">
      <TopNav
        label="SyncBox"
        heading={<Heading level={1}>SyncBox</Heading>}
        endContent={
          <Button
            label="Sync"
            tooltip="Sync the Rekordbox Library into SyncBox"
            variant="secondary"
            size="sm"
            isLoading={isSyncing}
            onClick={library.startSync}
          />
        }
      />
      {isSyncing && (
        <ProgressBar
          label={progressLabel(sync.progress)}
          value={overallProgress(sync.progress) * 100}
          hasValueLabel
        />
      )}
      {/* Player placeholder until playback lands (#4). */}
      <section className="sb-player" aria-label="Player">
        <Text type="supporting">No Track loaded</Text>
      </section>
    </div>
  );

  const banner =
    sync.status === "error" ? (
      <Banner
        status="error"
        container="section"
        title="Sync failed"
        description={sync.error.message}
        collapsible={false}
        isDismissable
        onDismiss={library.dismissSyncError}
      />
    ) : library.loadError ? (
      <Banner
        status="error"
        container="section"
        title="Could not read the Local DB"
        description={library.loadError}
        collapsible={false}
      />
    ) : undefined;

  return (
    <Theme theme={gothicTheme} mode="dark">
      <AppShell
        height="fill"
        contentPadding={0}
        topNav={topNav}
        banner={banner}
        mobileNav={false}
        sideNav={
          <PlaylistSidebar
            tree={tree}
            selectedId={selectedId}
            onSelect={library.select}
            isSyncing={isSyncing}
          />
        }
      >
        <div className="sb-main">
          <div className="sb-toolbar">
            <Heading level={2}>{listName}</Heading>
            <div className="sb-toolbar-end">
              <Text type="supporting">
                {count} {rows.length === 1 ? "Track" : "Tracks"}
              </Text>
              <TextInput
                label="Filter"
                isLabelHidden
                placeholder="Filter by title, artist or album"
                value={filter}
                onChange={setFilter}
                hasClear
                size="sm"
                startIcon="search"
              />
            </div>
          </div>
          <TrackTable
            rows={rows}
            isLoading={library.isLoadingTracks}
            emptyTitle={
              isFiltered
                ? "No Tracks match the filter"
                : isSyncing
                  ? "Syncing…"
                  : "No Tracks here"
            }
            emptyDescription={
              isFiltered
                ? "Try fewer or different words."
                : tree.length === 0 && !isSyncing
                  ? "Sync to bring in your Rekordbox Library."
                  : undefined
            }
          />
        </div>
      </AppShell>
    </Theme>
  );
}

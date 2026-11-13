import { EmptyState } from "@astryxdesign/core/EmptyState";
import {
  pixel,
  proportional,
  Table,
  type TableColumn,
  type TablePlugin,
  useTableColumnResize,
  useTableRowsVirtualization,
  useTableSortable,
} from "@astryxdesign/core/Table";
import { useMemo, useState } from "react";
import {
  type SortEntry,
  sortRows,
  type TrackTableRow,
} from "../../library/trackRows";

// Every column uses the default text renderer: a renderCell would switch
// Astryx Table from row windowing to rendering every row.
const COLUMNS: TableColumn<TrackTableRow>[] = [
  {
    key: "position",
    header: "#",
    width: pixel(56),
    align: "end",
    sortable: true,
  },
  {
    key: "title",
    header: "Title",
    width: proportional(3, { minWidth: 160 }),
    sortable: true,
  },
  {
    key: "artist",
    header: "Artist",
    width: proportional(2, { minWidth: 120 }),
    sortable: true,
  },
  {
    key: "album",
    header: "Album",
    width: proportional(2, { minWidth: 120 }),
    sortable: true,
  },
  { key: "bpm", header: "BPM", width: pixel(72), align: "end", sortable: true },
  { key: "key", header: "Key", width: pixel(64), sortable: true },
  {
    key: "length",
    header: "Length",
    width: pixel(76),
    align: "end",
    sortable: true,
  },
  {
    key: "bitrate",
    header: "Bitrate",
    width: pixel(76),
    align: "end",
    sortable: true,
  },
  { key: "rating", header: "Rating", width: pixel(84), sortable: true },
  { key: "status", header: "Status", width: pixel(88), sortable: true },
];

/**
 * Dims unplayable rows and shows their Unplayable Reason as the row tooltip,
 * and loads a Track on double-click.
 */
function rowBehaviour(
  onLoad: (row: TrackTableRow) => void,
): TablePlugin<TrackTableRow> {
  return {
    rowModel: "flat",
    transformBodyRow: (props, item) => ({
      ...props,
      htmlProps: {
        ...props.htmlProps,
        onDoubleClick: () => onLoad(item),
        ...(item.unplayable && {
          className: [props.htmlProps.className, "sb-row-unplayable"]
            .filter(Boolean)
            .join(" "),
          title: item.unplayable.message,
        }),
      },
    }),
  };
}

/** Shows the Rekordbox key name when hovering the Camelot key. */
const keyNameTooltip: TablePlugin<TrackTableRow> = {
  rowModel: "flat",
  transformBodyCell: (props, column, item) =>
    column.key === "key" && item.keyName
      ? {
          ...props,
          htmlProps: {
            ...props.htmlProps,
            title: `Rekordbox key: ${item.keyName}`,
          },
        }
      : props,
};

const WIDTHS_KEY = "syncbox.trackTable.columnWidths";

function readWidths(): Record<string, number> {
  try {
    const parsed = JSON.parse(localStorage.getItem(WIDTHS_KEY) ?? "{}");
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function writeWidths(widths: Record<string, number>) {
  try {
    localStorage.setItem(WIDTHS_KEY, JSON.stringify(widths));
  } catch {
    // Remembering column widths is a convenience only.
  }
}

/** Compact density row pitch, including the row divider. */
const ROW_HEIGHT = 33;

interface Props {
  rows: TrackTableRow[];
  isLoading: boolean;
  emptyTitle: string;
  emptyDescription?: string;
  onLoad: (row: TrackTableRow) => void;
}

export function TrackTable({
  rows,
  isLoading,
  emptyTitle,
  emptyDescription,
  onLoad,
}: Props) {
  const [sort, setSort] = useState<SortEntry[]>([]);
  const [columnWidths, setColumnWidths] = useState(readWidths);

  const sortable = useTableSortable<TrackTableRow, SortEntry["sortKey"]>({
    sort,
    onSortChange: setSort,
    allowUnsortedState: true,
  });
  const columnResize = useTableColumnResize<TrackTableRow>({
    // The resize config isn't generic over the row type.
    columns: COLUMNS as unknown as TableColumn<Record<string, unknown>>[],
    columnWidths,
    onColumnResizeEnd: (updates) =>
      setColumnWidths((prev) => {
        const next = { ...prev, ...updates };
        writeWidths(next);
        return next;
      }),
  });
  const virtualization = useTableRowsVirtualization({
    rowHeight: ROW_HEIGHT,
    initialViewportHeight: 800,
  });

  const sorted = useMemo(() => sortRows(rows, sort), [rows, sort]);
  const behaviour = useMemo(() => rowBehaviour(onLoad), [onLoad]);

  if (!isLoading && rows.length === 0) {
    return (
      <div className="sb-table-empty">
        <EmptyState title={emptyTitle} description={emptyDescription} />
      </div>
    );
  }

  return (
    <div className="sb-table">
      <Table
        data={sorted}
        columns={COLUMNS}
        idKey="id"
        density="compact"
        textOverflow="truncate"
        hasHover
        virtualization={virtualization}
        plugins={{ sortable, columnResize, keyNameTooltip, behaviour }}
      />
    </div>
  );
}

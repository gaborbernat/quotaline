import type {PanelProvider, SnapshotRow, StatusCell} from './service-state.js';

export function snapshotStale(rows: SnapshotRow[], now: number): boolean {
    return rows.length > 0 && rows.every(row => row.stale || !row.updated || now - row.updated > 3600);
}

export function snapshotRows(cells: StatusCell[], providers: Record<string, PanelProvider>): SnapshotRow[] {
    return cells.flatMap(cell => {
        const provider = providers[cell.provider];
        const window = provider?.windows.find(row => row.id === cell.id.slice(cell.provider.length + 1));
        return provider && window
            ? [
                  {
                      id: cell.id,
                      provider: provider.name,
                      label: window.label,
                      used: window.used,
                      reset: window.reset,
                      updated: provider.updated,
                      stale: provider.status === 'stale',
                  },
              ]
            : [];
    });
}

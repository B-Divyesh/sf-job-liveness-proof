export type RunState = 'completed' | 'running' | 'late' | 'missed' | 'failed' | 'contradictory';
export interface LedgerRow {
  job_key: string; display_name: string; run_id: string; scheduled_at: string;
  started_at: string | null; finished_at: string | null; completion_count: number | null;
  state: RunState; source: string | null; observed_status: string | null;
  source_url: string | null; observed_at: string | null; receipt_hash: string | null; is_virtual: boolean;
}
export interface Ledger { generated_at: string; rows: LedgerRow[]; summary: Partial<Record<RunState, number>>; }
export const states: RunState[] = ['contradictory','missed','late','failed','running','completed'];
export function filterRows(rows: LedgerRow[], state: string, query: string): LedgerRow[] {
  const needle = query.trim().toLocaleLowerCase();
  return rows.filter(row => (state === 'all' || row.state === state) && (!needle || `${row.display_name} ${row.job_key} ${row.run_id}`.toLocaleLowerCase().includes(needle)));
}
export function contradictionLabel(row: Pick<LedgerRow,'state'|'observed_status'>): string {
  if (row.state !== 'contradictory') return row.state;
  return row.observed_status === 'passed' ? 'Source passed, run did not' : 'Run completed, source did not';
}

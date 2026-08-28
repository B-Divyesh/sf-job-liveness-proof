import { describe, expect, it } from 'vitest';
import { contradictionLabel, filterRows, type LedgerRow } from './model';
const row = (state: LedgerRow['state'], name='Billing sweep'): LedgerRow => ({job_key:'billing',display_name:name,run_id:'r1',scheduled_at:'2026-01-01T00:00:00Z',started_at:null,finished_at:null,completion_count:null,state,source:null,observed_status:null,source_url:null,observed_at:null,receipt_hash:null,is_virtual:false});
describe('ledger model',()=>{
  it('filters status and query together',()=>expect(filterRows([row('missed'),row('completed','Digest')],'missed','bill')).toHaveLength(1));
  it('explains a contradictory passed source',()=>expect(contradictionLabel({...row('contradictory'),observed_status:'passed'})).toContain('passed'));
});

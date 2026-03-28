# Public Beta Rollback Runbook

## Trigger Conditions
- Any data-loss/P0 incident in canary.
- Restore checksum mismatch in automated matrix.
- Reproducible scheduler deadlock/race in critical flow.

## Immediate Actions (0-30 min)
1. Stop canary expansion immediately.
2. Disable canary verify flow:
   - `beta_canary_verify_enabled = false`
3. Announce incident in internal channel with:
   - Start time
   - Impacted cohort
   - First failing build/version

## Stabilization (30-120 min)
1. Collect diagnostics from impacted devices:
   - Settings > Beta Diagnostics > Export Diagnostics
2. Pull last logs (Verification/Failed/Skipped) for impacted destinations.
3. Identify regression window:
   - Compare latest known-good tag vs failing tag.

## Rollback Execution
1. Revert deploy to last known-good build.
2. Re-run smoke checks on reverted build:
   - Backup
   - Verify
   - Restore
3. Confirm no new failing checksum scenario in canary matrix.

## Recovery & Re-Enable Criteria
- Root cause identified and patched.
- Patch validated with:
  - `rtk cargo test --manifest-path src-tauri/Cargo.toml`
  - `rtk vitest run`
  - `rtk tsc --noEmit`
  - Integration verify/restore matrix pass
- Re-enable canary gate only for a small subset first.

## Postmortem Template
- Incident summary
- User impact
- Timeline (UTC)
- Root cause
- Fix and validation
- Preventive actions

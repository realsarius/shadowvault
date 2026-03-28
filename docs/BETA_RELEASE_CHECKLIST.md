# Public Beta Release Checklist (Canary)

## 1. Preflight
- Confirm branch is green:
  - `rtk cargo test --manifest-path src-tauri/Cargo.toml`
  - `rtk vitest run`
  - `rtk tsc --noEmit`
- Confirm restore verification matrix is green (local + en az 2 remote hedef).
- Confirm release notes include reliability and canary scope.

## 2. Canary Gate
- Verify `beta_canary_verify_enabled` setting strategy:
  - Canary cohort: `true`
  - Cohort dışı: `false` (gerektiğinde)
- Validate `verify_backup` works in canary and is blocked outside canary when disabled.

## 3. Operational Readiness
- Export and review diagnostics on test machine:
  - Settings > Beta Diagnostics > Export Diagnostics
- Confirm logs contain:
  - `trigger = Verification`
  - `status = Verified` and `status = Skipped` with reason
- Confirm overlapping scheduler jobs are skipped with DB log entry.

## 4. Canary Rollout (14 Days)
- Roll out to limited cohort first.
- Daily checks:
  - P0/data loss incident count
  - Verification checksum pass rate
  - Reproducible deadlock/race reports
- Success criteria:
  - P0/data loss = 0
  - Restore checksum = 100%
  - Reproducible deadlock/race = 0

## 5. Promote / Hold
- Promote to wider cohort only if all criteria pass.
- If any P0 or checksum mismatch appears, stop rollout and follow rollback runbook.

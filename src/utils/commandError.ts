export type RestoreErrorCode =
  | "blocked_path"
  | "missing_snapshot"
  | "wrong_password"
  | "chain_incomplete"
  | "io_failure"
  | "vault_locked"
  | "not_found"
  | "invalid_input"
  | "concurrency_conflict";

export interface ParsedCommandError {
  message: string;
  error_code: RestoreErrorCode | null;
}

export function parseCommandError(err: unknown): ParsedCommandError {
  const fallback = err instanceof Error ? err.message : String(err ?? "Unknown error");
  try {
    const parsed = JSON.parse(fallback) as { message?: string; error_code?: string };
    const errorCode =
      parsed?.error_code &&
      [
        "blocked_path",
        "missing_snapshot",
        "wrong_password",
        "chain_incomplete",
        "io_failure",
        "vault_locked",
        "not_found",
        "invalid_input",
        "concurrency_conflict",
      ].includes(parsed.error_code)
        ? (parsed.error_code as RestoreErrorCode)
        : null;
    return {
      message: parsed?.message ?? fallback,
      error_code: errorCode,
    };
  } catch {
    return {
      message: fallback,
      error_code: null,
    };
  }
}

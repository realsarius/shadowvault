import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

// Re-import after mock is set up
import { api } from "../api/tauri";

describe("api layer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("api.sources.getAll calls get_sources", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([]);
    const result = await api.sources.getAll();
    expect(invoke).toHaveBeenCalledWith("get_sources");
    expect(result).toEqual([]);
  });

  it("api.sources.create passes correct args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ id: "1", name: "test" });
    await api.sources.create("MySource", "/home/user/docs", "Directory");
    expect(invoke).toHaveBeenCalledWith("create_source", {
      name: "MySource",
      path: "/home/user/docs",
      sourceType: "Directory",
    });
  });

  it("api.jobs.runNow passes destinationId", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null);
    await api.jobs.runNow("dest-123");
    expect(invoke).toHaveBeenCalledWith("run_now", { destinationId: "dest-123" });
  });

  it("api.logs.clearOld passes olderThanDays", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(5);
    const count = await api.logs.clearOld(30);
    expect(invoke).toHaveBeenCalledWith("clear_old_logs", { olderThanDays: 30 });
    expect(count).toBe(5);
  });

  it("api.logs.count passes filters", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(10);
    const total = await api.logs.count({ sourceId: "src-1", status: "Failed", searchText: "disk full" });
    expect(invoke).toHaveBeenCalledWith("get_log_count", {
      sourceId: "src-1",
      status: "Failed",
      searchText: "disk full",
    });
    expect(total).toBe(10);
  });

  it("api.logs.deleteEntry passes logId", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(1);
    const deleted = await api.logs.deleteEntry(42);
    expect(invoke).toHaveBeenCalledWith("delete_log_entry", { logId: 42 });
    expect(deleted).toBe(1);
  });

  it("api.logs.export passes format and filters", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("/tmp/shadowvault-logs.csv");
    const path = await api.logs.export("csv", { status: "Failed", sourceId: "src-1" });
    expect(invoke).toHaveBeenCalledWith("export_logs", {
      format: "csv",
      status: "Failed",
      sourceId: "src-1",
    });
    expect(path).toBe("/tmp/shadowvault-logs.csv");
  });

  it("api.updater.check calls check_update", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ available: false, version: null, body: null });
    const info = await api.updater.check();
    expect(invoke).toHaveBeenCalledWith("check_update");
    expect(info.available).toBe(false);
  });
});

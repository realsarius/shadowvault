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

  it("api.updater.check calls check_update", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ available: false, version: null, body: null });
    const info = await api.updater.check();
    expect(invoke).toHaveBeenCalledWith("check_update");
    expect(info.available).toBe(false);
  });
});

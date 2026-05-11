import { mockCommands } from "../../test/tauri-mock";
import { DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS } from "../constants";
import { spawnClaude } from "../spawn-claude";

describe("spawnClaude", () => {
  it("invokes claude_spawn with the repoName when provided", async () => {
    const calls: Array<{ repoName: string | null; cols: number; rows: number }> = [];
    mockCommands({
      claude_spawn: (args) => {
        calls.push({
          repoName: args.repoName,
          cols: args.cols,
          rows: args.rows,
        });
        return {
          id: "fake-id",
          title: `Claude · ${args.repoName}`,
          status: "running",
          worktreePath: null,
          issueUrl: null,
          branch: null,
          repoName: args.repoName,
        };
      },
    });

    await spawnClaude("alpha");
    expect(calls).toHaveLength(1);
    expect(calls[0].repoName).toBe("alpha");
    expect(calls[0].cols).toBe(DEFAULT_PTY_COLS);
    expect(calls[0].rows).toBe(DEFAULT_PTY_ROWS);
  });

  it("passes null when called without a repoName (footer scratch)", async () => {
    const calls: Array<{ repoName: string | null }> = [];
    mockCommands({
      claude_spawn: (args) => {
        calls.push({ repoName: args.repoName });
        return {
          id: "fake-id",
          title: "Claude",
          status: "running",
          worktreePath: null,
          issueUrl: null,
          branch: null,
          repoName: null,
        };
      },
    });

    await spawnClaude();
    expect(calls).toHaveLength(1);
    expect(calls[0].repoName).toBeNull();
  });

  it("swallows errors and logs them rather than throwing", async () => {
    const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    mockCommands({
      claude_spawn: () => {
        throw "unknown repo";
      },
    });
    await expect(spawnClaude("ghost")).resolves.toBeUndefined();
    expect(errSpy).toHaveBeenCalled();
    errSpy.mockRestore();
  });
});

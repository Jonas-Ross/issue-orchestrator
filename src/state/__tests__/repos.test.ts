import { mockCommands } from "../../test/tauri-mock";
import { createReposStore, STORAGE_KEY as DRAWERS_KEY } from "../repos";
import type { RepoEntry } from "../../lib/bindings";

const r = (name: string, path = `/p/${name}`): RepoEntry => ({ name, path });

describe("repos store — drawer expansion", () => {
  it("defaults missing repo names to expanded (true)", () => {
    const { isDrawerExpanded } = createReposStore();
    expect(isDrawerExpanded("alpha")).toBe(true);
  });

  it("toggleDrawer persists state to storage", () => {
    const { toggleDrawer, isDrawerExpanded } = createReposStore();
    toggleDrawer("alpha");
    expect(isDrawerExpanded("alpha")).toBe(false);
    const persisted = JSON.parse(localStorage.getItem(DRAWERS_KEY) ?? "{}");
    expect(persisted.alpha).toBe(false);
  });

  it("loads persisted drawer state at construction", () => {
    localStorage.setItem(DRAWERS_KEY, JSON.stringify({ alpha: false }));
    const { isDrawerExpanded } = createReposStore();
    expect(isDrawerExpanded("alpha")).toBe(false);
  });

  it("recovers from corrupt JSON in storage", () => {
    localStorage.setItem(DRAWERS_KEY, "{not json");
    const { isDrawerExpanded } = createReposStore();
    expect(isDrawerExpanded("anything")).toBe(true);
  });
});

describe("repos store — IPC commands", () => {
  it("loadRepos populates the signal from list_repos", async () => {
    mockCommands({
      list_repos: () => [r("alpha"), r("beta")],
    });
    const { repos, loadRepos } = createReposStore();
    await loadRepos();
    expect(repos.value.map((x) => x.name)).toEqual(["alpha", "beta"]);
  });

  it("addRepoByPath calls add_repo then refreshes via list_repos", async () => {
    let listCalls = 0;
    mockCommands({
      add_repo: (args) => r(args.path.split("/").pop() ?? "x", args.path),
      list_repos: () => {
        listCalls++;
        return [r("alpha")];
      },
    });
    const { repos, addRepoByPath } = createReposStore();
    const added = await addRepoByPath("/repos/alpha");
    expect(added.name).toBe("alpha");
    expect(listCalls).toBe(1);
    expect(repos.value).toHaveLength(1);
  });

  it("removeRepoByName calls remove_repo then refreshes", async () => {
    mockCommands({
      remove_repo: () => null,
      list_repos: () => [],
    });
    const { repos, removeRepoByName } = createReposStore();
    await removeRepoByName("alpha");
    expect(repos.value).toEqual([]);
  });

  it("addRepoByPath surfaces backend errors as thrown Errors", async () => {
    // Tauri's typedError wrapper turns thrown non-Errors into result.error.
    // The mocked handler throws a string, which @tauri-apps/api/mocks
    // surfaces verbatim through invoke's rejection.
    mockCommands({
      add_repo: () => {
        throw "path is not a git repo";
      },
    });
    const { addRepoByPath } = createReposStore();
    await expect(addRepoByPath("/bad")).rejects.toThrow("path is not a git repo");
  });
});

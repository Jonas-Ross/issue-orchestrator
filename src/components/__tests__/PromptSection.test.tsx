import { fireEvent, render, screen, waitFor } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { repos } from "../../state/repos";
import { PromptSection } from "../SettingsPanel/PromptSection";

function configResult(template: string | null) {
  // Raw payload; `commands.getConfig` wraps via `typedError`.
  return {
    version: 1,
    worktreeRoot: "~/wt",
    repos: [{ name: "alpha", path: "/x" }],
    spawnPromptTemplate: template,
    setupDone: true,
  };
}

beforeEach(() => {
  // The repo selector reads the global signal, so seed it.
  repos.value = [{ name: "alpha", path: "/x" }];
});

describe("<PromptSection />", () => {
  it("loads the saved template into the textarea on mount", async () => {
    mockCommands({ get_config: () => configResult("Saved template") });
    render(<PromptSection />);
    const ta = (await screen.findByRole("textbox")) as HTMLTextAreaElement;
    expect(ta.value).toBe("Saved template");
  });

  it("falls back to the built-in default when no template is saved", async () => {
    mockCommands({ get_config: () => configResult(null) });
    render(<PromptSection />);
    const ta = (await screen.findByRole("textbox")) as HTMLTextAreaElement;
    expect(ta.value).toContain("issue-team");
    expect(ta.value).toContain("{issue_id}");
  });

  it("Save persists the template via update_spawn_prompt", async () => {
    let saved: { template: string | null } | undefined;
    mockCommands({
      get_config: () => configResult(null),
      update_spawn_prompt: (args: { template: string | null }) => {
        saved = args;
        return null;
      },
    });
    render(<PromptSection />);
    const ta = (await screen.findByRole("textbox")) as HTMLTextAreaElement;
    fireEvent.input(ta, { target: { value: "New custom #{issue_number}" } });
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(saved?.template).toBe("New custom #{issue_number}"));
    await screen.findByText("Saved.");
  });

  it("Reset to default sends null and refills the textarea", async () => {
    let lastTemplate: string | null | undefined;
    mockCommands({
      get_config: () => configResult("Old template"),
      update_spawn_prompt: (args: { template: string | null }) => {
        lastTemplate = args.template;
        return null;
      },
    });
    render(<PromptSection />);
    const ta = (await screen.findByRole("textbox")) as HTMLTextAreaElement;
    expect(ta.value).toBe("Old template");
    fireEvent.click(screen.getByText("Reset to default"));
    await waitFor(() => expect(lastTemplate).toBeNull());
    expect(ta.value).toContain("issue-team");
  });

  it("Optimize replaces the textarea contents on success", async () => {
    mockCommands({
      get_config: () => configResult("Original"),
      optimize_spawn_prompt: () => "Rewritten by claude",
    });
    render(<PromptSection />);
    const ta = (await screen.findByRole("textbox")) as HTMLTextAreaElement;
    expect(ta.value).toBe("Original");
    fireEvent.click(screen.getByText("Optimize with Claude"));
    await waitFor(() => expect(ta.value).toBe("Rewritten by claude"));
  });

  it("Optimize surfaces the error message when claude -p fails", async () => {
    mockCommands({
      get_config: () => configResult("Original"),
      optimize_spawn_prompt: () => {
        throw "boom";
      },
    });
    render(<PromptSection />);
    await screen.findByRole("textbox");
    fireEvent.click(screen.getByText("Optimize with Claude"));
    await screen.findByText(/Optimize failed: boom/);
  });
});

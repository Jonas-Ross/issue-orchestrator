import { DEFAULT_SPAWN_PROMPT, renderPrompt } from "../spawn-prompt";

describe("renderPrompt", () => {
  it("interpolates the default template", () => {
    expect(renderPrompt(DEFAULT_SPAWN_PROMPT, 7, "Add tab strip")).toBe(
      "Use the issue-team skill to implement issue #7 (Add tab strip).",
    );
  });

  it("replaces both placeholders in custom templates", () => {
    expect(renderPrompt("Implement {issue_title} (#{issue_number}).", 42, "Auth")).toBe(
      "Implement Auth (#42).",
    );
  });

  it("passes templates without placeholders through unchanged", () => {
    expect(renderPrompt("just do it", 1, "ignored")).toBe("just do it");
  });

  it("replaces multiple occurrences of the same placeholder", () => {
    expect(renderPrompt("#{issue_number} is #{issue_number}", 5, "x")).toBe("#5 is #5");
  });
});

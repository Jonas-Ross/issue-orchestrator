import { DEFAULT_SPAWN_PROMPT, renderPrompt } from "../spawn-prompt";

describe("renderPrompt", () => {
  it("interpolates the default template", () => {
    expect(renderPrompt(DEFAULT_SPAWN_PROMPT, "7", "Add tab strip")).toBe(
      "Use the issue-team skill to implement issue #7 (Add tab strip).",
    );
  });

  it("replaces both placeholders in custom templates", () => {
    expect(renderPrompt("Implement {issue_title} (#{issue_id}).", "42", "Auth")).toBe(
      "Implement Auth (#42).",
    );
  });

  it("substitutes provider-style ids verbatim", () => {
    expect(renderPrompt("Work on {issue_id}", "PROJ-7", "x")).toBe("Work on PROJ-7");
  });

  it("accepts {issue_number} as a back-compat alias for {issue_id}", () => {
    expect(renderPrompt("Old (#{issue_number})", "ENG-9", "x")).toBe("Old (#ENG-9)");
  });

  it("passes templates without placeholders through unchanged", () => {
    expect(renderPrompt("just do it", "1", "ignored")).toBe("just do it");
  });

  it("replaces multiple occurrences of the same placeholder", () => {
    expect(renderPrompt("#{issue_id} is #{issue_id}", "5", "x")).toBe("#5 is #5");
  });
});

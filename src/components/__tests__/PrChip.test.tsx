import { fireEvent, render, screen } from "@testing-library/preact";
import { mockCommands } from "../../test/tauri-mock";
import { PrChip } from "../PrChip";
import type { PrStatus } from "../../lib/bindings";

beforeEach(() => {
  mockCommands({});
});

function makePr(overrides: Partial<PrStatus> = {}): PrStatus {
  return {
    number: 128,
    url: "https://github.com/foo/bar/pull/128",
    checks: "pass",
    ...overrides,
  };
}

describe("<PrChip />", () => {
  it.each([
    ["pass", "pr-chip-pass"],
    ["fail", "pr-chip-fail"],
    ["pending", "pr-chip-pending"],
    ["none", "pr-chip-none"],
  ] as const)("applies the %s class for checks=%s", (checks, cls) => {
    const { container } = render(<PrChip prStatus={makePr({ checks })} />);
    expect(container.firstChild).toHaveClass(cls);
  });

  it("renders the PR number in the label", () => {
    render(<PrChip prStatus={makePr({ number: 42 })} />);
    expect(screen.getByText(/PR #42/)).toBeInTheDocument();
  });

  it("clicking calls open with the PR URL", () => {
    let openedUrl: string | undefined;
    mockCommands({
      "plugin:shell|open": (args: { path: string }) => {
        openedUrl = args.path;
        return null;
      },
    });
    const pr = makePr({ url: "https://github.com/foo/bar/pull/42", number: 42 });
    const { container } = render(<PrChip prStatus={pr} />);
    const btn = container.querySelector(".pr-chip") as HTMLElement;
    expect(btn).not.toBeNull();
    fireEvent.click(btn);
    expect(openedUrl).toBe("https://github.com/foo/bar/pull/42");
  });

  it("clicking does not bubble to parent", () => {
    mockCommands({
      "plugin:shell|open": () => null,
    });
    const parentClicked = { called: false };
    const pr = makePr();
    const { container } = render(
      <div onClick={() => (parentClicked.called = true)}>
        <PrChip prStatus={pr} />
      </div>,
    );
    const btn = container.querySelector(".pr-chip") as HTMLElement;
    fireEvent.click(btn);
    expect(parentClicked.called).toBe(false);
  });
});

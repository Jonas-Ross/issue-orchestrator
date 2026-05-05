import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { commands } from "../lib/bindings";
import { activeId, sessions } from "../state/sessions";
import { openPicker } from "../state/picker";
import { closePalette, paletteOpen } from "../state/palette";

interface PaletteAction {
  id: string;
  label: string;
  hint?: string;
  run: () => void;
}

export function CommandPalette() {
  if (!paletteOpen.value) return null;

  const [query, setQuery] = useState("");
  const [activeIdx, setActiveIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const all: PaletteAction[] = useMemo(() => {
    const out: PaletteAction[] = [];
    out.push({
      id: "new-issue",
      label: "New issue session",
      hint: "⌘N",
      run: () => {
        closePalette();
        openPicker();
      },
    });
    if (activeId.value) {
      out.push({
        id: "kill-active",
        label: "Kill active session",
        hint: "⌘W",
        run: () => {
          const id = activeId.value;
          if (id) void commands.ptyKill(id);
          closePalette();
        },
      });
    }
    for (const s of sessions.value) {
      out.push({
        id: `switch-${s.id}`,
        label: `Switch to: ${s.title}`,
        hint: s.id === activeId.value ? "active" : undefined,
        run: () => {
          activeId.value = s.id;
          closePalette();
        },
      });
    }
    return out;
    // Reading `signal.value` inside the memo IS the reactive subscription
    // we want; oxlint's react-hooks/exhaustive-deps doesn't recognize that
    // pattern and flags `sessions` / `activeId` as unnecessary.
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, [sessions.value, activeId.value]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return all;
    const scored = all
      .map((a) => {
        const label = a.label.toLowerCase();
        if (label.startsWith(q)) return { a, score: 0 };
        if (label.includes(q)) return { a, score: 1 };
        return { a, score: -1 };
      })
      .filter((x) => x.score >= 0)
      .sort((x, y) => x.score - y.score);
    return scored.map((x) => x.a);
  }, [all, query]);

  useEffect(() => {
    setActiveIdx((i) => (i >= filtered.length ? 0 : i));
  }, [filtered.length]);

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIdx((i) => Math.min(i + 1, Math.max(0, filtered.length - 1)));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIdx((i) => Math.max(0, i - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      filtered[activeIdx]?.run();
    }
  };

  return (
    <div class="modal-overlay" onClick={() => closePalette()}>
      <div class="modal" style={{ width: 480 }} onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          class="palette-input"
          placeholder="Switch session, run command, open issue…"
          value={query}
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
          onKeyDown={onKeyDown}
        />
        <ul class="palette-list">
          {filtered.length === 0 && (
            <li class="hint" style={{ background: "transparent", color: "#888" }}>
              No matches.
            </li>
          )}
          {filtered.map((a, i) => (
            <li
              key={a.id}
              class={i === activeIdx ? "active" : undefined}
              onMouseEnter={() => setActiveIdx(i)}
              onClick={() => a.run()}
            >
              <span>{a.label}</span>
              {a.hint && (
                <span style={{ float: "right", opacity: 0.6, fontSize: 11 }}>{a.hint}</span>
              )}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

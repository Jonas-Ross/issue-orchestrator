import { makeSession } from "../../test/factories";
import { createSessionsState, SHELL_BUCKET } from "../sessions";

describe("sessions state", () => {
  it("starts with no sessions and no active id", () => {
    const { sessions, activeId } = createSessionsState();
    expect(sessions.value).toEqual([]);
    expect(activeId.value).toBeNull();
  });

  describe("addSession", () => {
    it("appends and auto-activates the first session", () => {
      const { sessions, activeId, addSession } = createSessionsState();
      addSession(makeSession());
      expect(sessions.value).toHaveLength(1);
      expect(activeId.value).toBe("s1");
    });

    it("does not overwrite the active id when adding subsequent sessions", () => {
      const { activeId, addSession } = createSessionsState();
      addSession(makeSession({ id: "s1" }));
      addSession(makeSession({ id: "s2" }));
      expect(activeId.value).toBe("s1");
    });

    it("dedups by id", () => {
      const { sessions, addSession } = createSessionsState();
      addSession(makeSession({ id: "s1" }));
      addSession(makeSession({ id: "s1", title: "duplicate" }));
      expect(sessions.value).toHaveLength(1);
      expect(sessions.value[0].title).toBe("Session 1");
    });
  });

  describe("removeSession", () => {
    it("drops the session and rolls activeId to next survivor", () => {
      const { sessions, activeId, addSession, removeSession } = createSessionsState();
      addSession(makeSession({ id: "s1" }));
      addSession(makeSession({ id: "s2" }));
      removeSession("s1");
      expect(sessions.value).toHaveLength(1);
      expect(activeId.value).toBe("s2");
    });

    it("sets activeId to null when removing the last session", () => {
      const { activeId, addSession, removeSession } = createSessionsState();
      addSession(makeSession({ id: "s1" }));
      removeSession("s1");
      expect(activeId.value).toBeNull();
    });

    it("preserves activeId when removing a non-active session", () => {
      const { activeId, addSession, removeSession } = createSessionsState();
      addSession(makeSession({ id: "s1" }));
      addSession(makeSession({ id: "s2" }));
      removeSession("s2");
      expect(activeId.value).toBe("s1");
    });
  });

  describe("setStatus", () => {
    it("updates only the matching session", () => {
      const { sessions, addSession, setStatus } = createSessionsState();
      addSession(makeSession({ id: "s1", status: "running" }));
      addSession(makeSession({ id: "s2", status: "running" }));
      setStatus("s1", "needs_input");
      expect(sessions.value[0].status).toBe("needs_input");
      expect(sessions.value[1].status).toBe("running");
    });

    it("is a no-op for unknown ids", () => {
      const { sessions, addSession, setStatus } = createSessionsState();
      addSession(makeSession({ id: "s1", status: "running" }));
      setStatus("nope", "exited");
      expect(sessions.value[0].status).toBe("running");
    });
  });

  describe("sessionsByRepo", () => {
    it("groups by repoName", () => {
      const { sessionsByRepo, addSession } = createSessionsState();
      addSession(makeSession({ id: "a1", repoName: "alpha" }));
      addSession(makeSession({ id: "a2", repoName: "alpha" }));
      addSession(makeSession({ id: "b1", repoName: "beta" }));
      const groups = sessionsByRepo.value;
      expect(groups.get("alpha")).toHaveLength(2);
      expect(groups.get("beta")).toHaveLength(1);
    });

    it("collects null repoName under SHELL_BUCKET", () => {
      const { sessionsByRepo, addSession } = createSessionsState();
      addSession(makeSession({ id: "shell1", repoName: null }));
      expect(sessionsByRepo.value.get(SHELL_BUCKET)).toHaveLength(1);
    });
  });

  it("isolated factories don't share state", () => {
    const a = createSessionsState();
    const b = createSessionsState();
    a.addSession(makeSession({ id: "s1" }));
    expect(a.sessions.value).toHaveLength(1);
    expect(b.sessions.value).toHaveLength(0);
  });
});

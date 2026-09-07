import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Bot, Send, UserRound } from "lucide-react";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { ViewId } from "./types";

// Layout-only fixture: production shell/styles, representative transcript markup.
// It never starts a provider or reads/writes application sessions or user files.
const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
mockWindows("main");
mockIPC(() => undefined);
const state = params.get("state") ?? "long";
const initialView = params.get("view") === "settings" ? "settings"
  : params.get("view") === "overview" ? "overview" : "assistant";
const count = state === "short" ? 2 : state === "empty" ? 0 : 16;
const longAnswer = [
  "이 문장은 레이아웃 검증용 예시입니다. 실제 파일 분석 결과가 아닙니다.",
  "총 크기, 파일 개수, 항목별 비율처럼 여러 줄의 설명을 읽는 상황을 재현합니다.",
  "긴_항목_이름_".repeat(24),
  "파일은 변경하지 않습니다. 제목이 본문 위에 겹치지 않아야 합니다.",
].join("\n\n");

function Fixture() {
  const [activeView, setActiveView] = useState<ViewId>(initialView);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [draft, setDraft] = useState("");
  const [sent, setSent] = useState(false);
  return <AppShell activeView={activeView} root={null} report={null} volume={null}
    mobileNavigationOpen={mobileOpen} selectionBlocked={false} dockerEnabled={false}
    onMobileNavigationChange={setMobileOpen} onNavigate={setActiveView} onPickFolder={() => {}}>
    <div className="assistant-workspace">
      <p role="note">레이아웃 테스트 화면 · 실제 AI 요청과 파일 작업 없음</p>
      <section className="assistant-scope" aria-label="테스트 대화 대상">
        <span>테스트 폴더 · 짧은 대화와 긴 대화</span>
        <label>테스트 공급자 <select defaultValue="fixture"><option value="fixture">테스트 전용</option></select></label>
      </section>
      <section className="assistant-chat" aria-label="테스트 대화">
        <div className="assistant-transcript">
          {Array.from({ length: count }, (_, index) => <article
            className={`assistant-message is-${index % 2 ? "assistant" : "user"}`} key={index}>
            <span aria-hidden="true">{index % 2 ? <Bot size={17} /> : <UserRound size={17} />}</span>
            <div><strong>{index % 2 ? "테스트 응답" : "나"} {index + 1}</strong>
              <p>{index % 2 ? longAnswer : "선택한 폴더의 크기와 검토 순서를 설명해주세요. 실제 작업은 하지 마세요."}</p></div>
          </article>)}
          {state === "empty" ? <p>테스트 대화가 비어 있습니다.</p> : null}
          {state === "loading" ? <p role="status">테스트 응답 대기 중…</p> : null}
          {state === "error" ? <p role="alert">테스트 오류입니다. 다시 시도할 수 있습니다.</p> : null}
        </div>
        <form className="assistant-composer" onSubmit={(event) => { event.preventDefault(); setSent(true); }}>
          <textarea aria-label="테스트 질문" placeholder="테스트 질문을 입력하세요…" value={draft}
            onChange={(event) => setDraft(event.target.value)} disabled={state === "loading"} />
          <button type="submit" aria-label="테스트 보내기" disabled={!draft.trim() || state === "loading"}><Send size={17} /></button>
        </form>
        <p className="assistant-composer-note" role="status">{sent ? "테스트 입력 확인 · 외부 전송 없음" : "레이아웃 검증용 · 외부 전송 없음"}</p>
      </section>
    </div>
  </AppShell>;
}

createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);

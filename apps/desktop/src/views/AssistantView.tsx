import { Bot, Laptop, Link2, ShieldCheck, Terminal } from "lucide-react";
import { ControlStatusPanel } from "../components/ControlStatusPanel";
import type { ControlStatus, ScanConfig } from "../types";

interface AssistantViewProps {
  status: ControlStatus;
  canEnableSearch: boolean;
  updatingSearchAccess: boolean;
  searchAccessError: string | null;
  onToggleSearchAccess: () => void;
  scanRoot: string | null;
  scanConfig: ScanConfig;
  canEnableScan: boolean;
  updatingScanAccess: boolean;
  scanAccessError: string | null;
  onToggleScanAccess: () => void;
}

export function AssistantView({
  status,
  canEnableSearch,
  updatingSearchAccess,
  searchAccessError,
  onToggleSearchAccess,
  scanRoot,
  scanConfig,
  canEnableScan,
  updatingScanAccess,
  scanAccessError,
  onToggleScanAccess,
}: AssistantViewProps) {
  return (
    <div className="view-stack assistant-view">
      <section className="assistant-intro" aria-labelledby="assistant-intro-title">
        <span className="assistant-intro__icon" aria-hidden="true">
          <Bot size={24} />
        </span>
        <div>
          <p className="eyebrow">기본 기능과 분리된 선택 기능</p>
          <h2 id="assistant-intro-title">
            대화는 설치된 CLI에서, 파일 작업은 BroomSweepy에서 합니다
          </h2>
          <p>
            이 앱 안에 별도 AI 대화상대가 생기는 방식이 아닙니다. Codex, Claude Code,
            Gemini CLI 같은 로컬 도구가 요청을 보내면 BroomSweepy가 실제 검색과 검사를
            수행합니다.
          </p>
        </div>
        <span className="assistant-intro__optional">
          <ShieldCheck size={16} aria-hidden="true" />
          연결하지 않아도 모든 기본 기능을 사용할 수 있습니다
        </span>
      </section>

      <ol className="assistant-steps" aria-label="로컬 CLI 연결 순서">
        <li>
          <span aria-hidden="true"><Laptop size={18} /></span>
          <div>
            <small>1단계</small>
            <strong>로컬 CLI 준비</strong>
            <p>지원하는 CLI를 이 PC에서 실행합니다.</p>
          </div>
        </li>
        <li>
          <span aria-hidden="true"><Link2 size={18} /></span>
          <div>
            <small>2단계</small>
            <strong>MCP 연결 도구 등록</strong>
            <p>CLI 설치만으로 자동 연결되지는 않습니다.</p>
          </div>
        </li>
        <li>
          <span aria-hidden="true"><Terminal size={18} /></span>
          <div>
            <small>3단계</small>
            <strong>CLI 대화창에서 요청</strong>
            <p>허용한 범위의 검색·검사만 앱에 전달됩니다.</p>
          </div>
        </li>
      </ol>

      <ControlStatusPanel
        status={status}
        canEnableSearch={canEnableSearch}
        updatingSearchAccess={updatingSearchAccess}
        searchAccessError={searchAccessError}
        onToggleSearchAccess={onToggleSearchAccess}
        scanRoot={scanRoot}
        scanConfig={scanConfig}
        canEnableScan={canEnableScan}
        updatingScanAccess={updatingScanAccess}
        scanAccessError={scanAccessError}
        onToggleScanAccess={onToggleScanAccess}
      />

      <section className="assistant-boundary" aria-labelledby="assistant-boundary-title">
        <div>
          <p className="eyebrow">무엇을 할 수 있나요?</p>
          <h2 id="assistant-boundary-title">검색과 검사는 요청할 수 있고, 삭제는 앱에서만 확인합니다</h2>
        </div>
        <ul>
          <li>앱이 미리 만든 파일 이름·문서 내용 목록 검색</li>
          <li>앱에서 이번 실행에 허용한 폴더의 저장공간 검사 시작·상태 확인·취소</li>
          <li>휴지통 이동과 레지스트리 변경은 외부 CLI에 공개하지 않음</li>
        </ul>
      </section>
    </div>
  );
}

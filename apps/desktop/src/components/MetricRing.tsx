import { useId } from "react";

interface MetricRingProps {
  label: string;
  value: number | null;
  valueText: string;
  detail: string;
  tone?: "cpu" | "memory";
}

export function MetricRing({ label, value, valueText, detail, tone = "cpu" }: MetricRingProps) {
  const gradientId = useId().replace(/:/g, "");
  const percent = value === null ? 0 : Math.min(100, Math.max(0, value));
  const radius = 86;
  const circumference = Math.PI * 2 * radius;
  const dashOffset = circumference * (1 - percent / 100);

  return (
    <div
      className={`metric-ring ${value === null ? "is-unavailable" : ""}`}
      role="meter"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value === null ? undefined : Math.round(percent)}
      aria-valuetext={valueText}
    >
      <svg viewBox="0 0 200 200" aria-hidden="true">
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="var(--accent-blue)" />
            <stop offset="1" stopColor={tone === "memory" ? "var(--accent-blue)" : "var(--accent-violet)"} />
          </linearGradient>
        </defs>
        <circle className="metric-ring__track" cx="100" cy="100" r={radius} />
        <circle
          className="metric-ring__value"
          cx="100"
          cy="100"
          r={radius}
          stroke={`url(#${gradientId})`}
          strokeDasharray={circumference}
          strokeDashoffset={dashOffset}
        />
      </svg>
      <span className="metric-ring__copy">
        <small>{label}</small>
        <strong>{valueText}</strong>
        <span>{detail}</span>
      </span>
    </div>
  );
}

interface OrbProps {
  size?: number;
  className?: string;
  thinking?: boolean;
}

export function Orb({ size = 140, className = "", thinking = false }: OrbProps) {
  return (
    <div
      className={`relative inline-block rounded-full shrink-0 ${thinking ? "animate-[orb-breathe_1.8s_ease-in-out_infinite]" : "animate-[orb-breathe_3.8s_ease-in-out_infinite]"} ${className}`}
      style={{ width: size, height: size }}
    >
      <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg" className="w-full h-full block">
        <defs>
          <radialGradient id="orbG" cx="38%" cy="30%" r="68%">
            <stop offset="0%" stopColor="#ffd4bc" />
            <stop offset="45%" stopColor="#D97757" />
            <stop offset="100%" stopColor="#8a3f2a" />
          </radialGradient>
          <radialGradient id="orbH" cx="35%" cy="28%" r="22%">
            <stop offset="0%" stopColor="#fff8ef" stopOpacity="0.95" />
            <stop offset="100%" stopColor="#fff8ef" stopOpacity="0" />
          </radialGradient>
          <filter id="orbGlow"><feGaussianBlur stdDeviation="10" /></filter>
        </defs>
        <circle cx="100" cy="100" r="92" fill="#D97757" opacity="0.15" filter="url(#orbGlow)" />
        <circle cx="100" cy="100" r="80" fill="url(#orbG)" />
        <ellipse cx="78" cy="66" rx="28" ry="18" fill="url(#orbH)" />
        <circle cx="138" cy="140" r="7" fill="#fff8ef" opacity="0.3" />
      </svg>
    </div>
  );
}

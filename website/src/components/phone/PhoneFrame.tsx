import { type ReactNode } from "react";

interface PhoneFrameProps {
  children: ReactNode;
  className?: string;
}

export function PhoneFrame({ children, className = "" }: PhoneFrameProps) {
  return (
    <div className={`phone-scope ${className}`}>
      <div className="phone-frame">
        {/* Notch */}
        <div className="phone-notch" />
        {/* Screen content */}
        <div className="phone-screen">
          {children}
        </div>
        {/* Home indicator */}
        <div className="phone-home-indicator" />
      </div>
    </div>
  );
}

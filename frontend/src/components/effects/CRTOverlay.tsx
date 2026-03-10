import { useCallback, useState } from "react";

/**
 * CRT scanline overlay — toggleable via the `enabled` prop.
 * Sets the `--crt-opacity` CSS variable so the effect can be
 * controlled from anywhere.
 */
export interface CRTOverlayProps {
  enabled?: boolean;
}

export default function CRTOverlay({ enabled = false }: CRTOverlayProps) {
  return (
    <div
      className="crt-overlay"
      data-testid="crt-overlay"
      style={{ opacity: enabled ? 0.4 : 0 }}
    />
  );
}

/**
 * Hook for toggling CRT effect on/off.
 */
export function useCRTToggle(initial = false) {
  const [enabled, setEnabled] = useState(initial);
  const toggle = useCallback(() => setEnabled((v) => !v), []);
  return { crtEnabled: enabled, toggleCRT: toggle } as const;
}

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  copyImage,
  errorMessage,
  saveImage,
  shareImage,
  type Insights,
  type ShareNetwork,
} from "../lib/ipc";
import { Dialog } from "./Dialog";
import { CARD_HEIGHT, CARD_WIDTH, drawShareCard, pngBytes } from "./shareCard";

/** The networks' own names, which aren't translated. */
const NETWORKS: [ShareNetwork, string][] = [
  ["x", "X"],
  ["linkedIn", "LinkedIn"],
];

interface ShareDialogProps {
  insights: Insights;
  onClose: () => void;
}

/** Copy image, Download, or share the stats card to X or LinkedIn (STA-03). */
export function ShareDialog({ insights, onClose }: ShareDialogProps) {
  const { t, i18n } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [drawn, setDrawn] = useState<boolean | null>(null);
  const [status, setStatus] = useState<{ text: string; error: boolean } | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const ok = canvas ? drawShareCard(canvas, insights, t, i18n.language) : false;
    const frame = requestAnimationFrame(() => setDrawn(ok));
    return () => cancelAnimationFrame(frame);
  }, [insights, t, i18n.language]);

  const run = (action: (png: Uint8Array) => Promise<string>) => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    pngBytes(canvas)
      .then(action)
      .then((text) => setStatus({ text, error: false }))
      .catch((err: unknown) => setStatus({ text: errorMessage(err), error: true }));
  };

  return (
    <Dialog title={t("share.title")} onClose={onClose} wide>
      <canvas
        ref={canvasRef}
        className="share-card"
        width={CARD_WIDTH}
        height={CARD_HEIGHT}
        role="img"
        aria-label={t("share.preview")}
      />
      {drawn === false && (
        <p className="error" role="alert">
          {t("share.drawFailed")}
        </p>
      )}
      <p className={status?.error ? "error" : "muted"} role="status">
        {status?.text}
      </p>
      <div className="dialog-actions">
        <button type="button" onClick={onClose}>
          {t("common.close")}
        </button>
        {NETWORKS.map(([network, name]) => (
          <button
            key={network}
            type="button"
            disabled={!drawn}
            aria-label={t("share.shareOn", { network: name })}
            onClick={() =>
              run((png) => shareImage(png, network).then(() => t("share.pasteIntoPost")))
            }
          >
            {name}
          </button>
        ))}
        <button
          type="button"
          disabled={!drawn}
          onClick={() =>
            run((png) =>
              saveImage(png, t("share.cardTitle")).then((path) => t("share.saved", { path })),
            )
          }
        >
          {t("share.download")}
        </button>
        <button
          type="button"
          className="primary"
          disabled={!drawn}
          onClick={() => run((png) => copyImage(png).then(() => t("share.copied")))}
        >
          {t("share.copy")}
        </button>
      </div>
    </Dialog>
  );
}

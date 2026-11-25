import { openUrl } from "@tauri-apps/plugin-opener";
import { Copy, ExternalLink, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { GlassPanel } from "@/components/ui/glass";
import { notify } from "@/components/ui/toast";

export const TAILSCALE_ACL_PAGE = "https://login.tailscale.com/admin/acls/file";

/**
 * Tailnet policy that only lets the group reach each other's game port.
 * Replacing the default policy also drops its allow-all rule and its SSH
 * rule: anything not listed is denied.
 */
export const ACL_POLICY = `{
  // Only the players, only Rocket League's game port (UDP 7777).
  "groups": {
    "group:rl": ["you@example.com", "friend@example.com"]
  },
  "acls": [
    { "action": "accept", "src": ["group:rl"], "proto": "udp", "dst": ["group:rl:7777"] }
  ],
  // No Tailscale SSH between your PCs.
  "ssh": []
}`;

export function SecurityPanel() {
  const { t } = useTranslation("multi");
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(ACL_POLICY);
      notify.success(t("session.copied"));
    } catch (err) {
      notify.error(err);
    }
  };
  const tips = ["acl", "noSsh", "noRoutes", "firewall", "cleanup"] as const;

  return (
    <GlassPanel className="flex flex-col gap-3 p-4">
      <div className="flex items-center gap-2">
        <ShieldCheck className="size-4 text-success" />
        <h2 className="text-sm font-semibold">{t("security.title")}</h2>
      </div>
      <p className="text-[13px] text-fg-muted">{t("security.intro")}</p>
      <ol className="flex list-decimal flex-col gap-1.5 pl-4 text-xs text-fg-muted">
        {tips.map((k) => (
          <li key={k}>{t(`security.tips.${k}`)}</li>
        ))}
      </ol>
      <pre className="overflow-x-auto rounded-[8px] border border-line bg-black/25 p-3 font-mono text-[11px] leading-relaxed text-fg-muted">
        {ACL_POLICY}
      </pre>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="secondary" onClick={copy}>
          <Copy />
          {t("security.copy")}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void openUrl(TAILSCALE_ACL_PAGE).catch(notify.error)}
        >
          <ExternalLink />
          {t("security.open")}
        </Button>
      </div>
    </GlassPanel>
  );
}

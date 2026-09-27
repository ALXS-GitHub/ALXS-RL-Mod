import { Link, useRouterState } from "@tanstack/react-router";
import { PanelLeftClose, PanelLeftOpen, ShieldCheck } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { NAV, NAV_SECTIONS } from "@/app/nav";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { useAppInfo } from "@/lib/game";
import { useUi } from "@/stores/ui";

export function Sidebar() {
  const { t } = useTranslation();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const collapsed = useUi((s) => s.sidebarCollapsed);
  const toggle = useUi((s) => s.toggleSidebar);
  const { data: info } = useAppInfo();

  const isActive = (to: string) => (to === "/" ? pathname === "/" : pathname.startsWith(to));

  return (
    <motion.nav
      animate={{ width: collapsed ? 56 : 220 }}
      transition={{ type: "spring", stiffness: 500, damping: 45 }}
      className="relative z-30 flex shrink-0 flex-col border-line border-r bg-[rgb(10_11_15/0.55)] backdrop-blur-xl"
    >
      <div className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-3">
        {NAV_SECTIONS.map((section) => {
          const items = NAV.filter((n) => n.section === section);
          return (
            <div key={section} className="mb-4">
              <p
                className={cn(
                  "mb-1 h-5 px-2.5 text-[11px] font-medium text-fg-subtle transition-opacity",
                  collapsed && "opacity-0",
                )}
              >
                {t(`nav.sections.${section}`)}
              </p>
              <ul className="flex flex-col gap-px">
                {items.map((item) => {
                  const active = isActive(item.to);
                  const Icon = item.icon;
                  const link = (
                    <Link
                      to={item.to}
                      className={cn(
                        "group relative flex h-8 items-center gap-2.5 rounded-[7px] px-2.5 text-[13px] font-medium transition-colors",
                        active ? "text-fg" : "text-fg-muted hover:bg-white/[0.04] hover:text-fg",
                      )}
                    >
                      {active ? (
                        <motion.span
                          layoutId="sidebar-active"
                          transition={{ type: "spring", stiffness: 500, damping: 42 }}
                          className="absolute inset-0 -z-10 rounded-[7px] bg-white/[0.08]"
                        />
                      ) : null}
                      <Icon
                        className={cn(
                          "size-4 shrink-0",
                          active ? "text-[var(--color-accent)]" : "text-fg-subtle group-hover:text-fg-muted",
                        )}
                      />
                      <span className={cn("truncate transition-opacity", collapsed && "opacity-0")}>
                        {t(item.labelKey)}
                      </span>
                      {item.stage && !collapsed ? (
                        <span className="ml-auto rounded-[4px] bg-warning/12 px-1 text-[10px] font-medium text-warning">
                          {t(`nav.stages.${item.stage}`)}
                        </span>
                      ) : null}
                    </Link>
                  );
                  return (
                    <li key={item.to}>
                      {collapsed ? (
                        <Tooltip content={t(item.labelKey)} side="right">
                          {link}
                        </Tooltip>
                      ) : (
                        link
                      )}
                    </li>
                  );
                })}
              </ul>
            </div>
          );
        })}
      </div>

      <div className="flex items-center justify-between gap-2 border-line border-t px-2 py-2">
        {!collapsed ? (
          <div className="flex min-w-0 items-center gap-1.5 px-1.5 text-[11px] text-fg-subtle">
            <ShieldCheck className="size-3.5 shrink-0" />
            <span className="truncate">
              v{info?.version ?? "…"} · {t("app.noInjection")}
            </span>
          </div>
        ) : null}
        <button
          type="button"
          onClick={toggle}
          aria-label="Toggle sidebar"
          className="grid size-7 shrink-0 place-items-center rounded-[6px] text-fg-subtle hover:bg-white/[0.06] hover:text-fg"
        >
          {collapsed ? <PanelLeftOpen className="size-4" /> : <PanelLeftClose className="size-4" />}
        </button>
      </div>
    </motion.nav>
  );
}

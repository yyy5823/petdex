"use client";

import { ArrowRight, Clock } from "lucide-react";

import {
  type MacArch,
  type Platform,
  useMacArch,
  usePlatform,
} from "@/lib/use-platform";

import { CommandLine } from "@/components/command-line";

const MACOS_ARM64_URL = "/api/desktop/latest-release?asset=darwin-arm64";
const MACOS_X64_URL = "/api/desktop/latest-release?asset=darwin-x64";

/**
 * The hero-row "Download for macOS" + CLI install CTA, rendered
 * differently per detected platform so we never offer a click that
 * dead-ends in a binary the user can't run.
 *
 *   macOS         → primary download button (direct binary)
 *   linux/win     → disabled "Coming soon" pill + still-works CLI note
 *                   (CLI itself runs on those platforms too, even if the
 *                   GUI binary isn't self-contained yet)
 *   ios/ipados    → "macOS-only desktop" coming-soon, no CTA at all
 *   android       → same as iOS
 *   unknown/other → neutral placeholder (SSR + first paint, or a
 *                   browser we couldn't classify)
 *
 * The CLI command line stays visible across every platform — the
 * `petdex install desktop` command just won't find a binary on
 * non-macOS today, but it returns a clear error rather than
 * pretending to install. That's still better DX than hiding the
 * mention entirely.
 */
export function DownloadCTA({
  primaryLabel,
  cliCommand,
  cliSubtext,
  comingSoonLabel,
  desktopOnlyLabel,
}: {
  primaryLabel: string;
  cliCommand: string;
  cliSubtext: string;
  comingSoonLabel: string;
  desktopOnlyLabel: string;
}) {
  const platform = usePlatform();
  const arch = useMacArch();

  return (
    <div className="mt-10 flex w-full flex-col items-center gap-3">
      <div className="flex w-full flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
        <PrimaryButton
          platform={platform}
          arch={arch}
          primaryLabel={primaryLabel}
          comingSoonLabel={comingSoonLabel}
          desktopOnlyLabel={desktopOnlyLabel}
        />
        <CommandLine
          command={cliCommand}
          source="download-hero"
          className="!h-12 w-full !rounded-full !px-5 !text-[13px] sm:w-auto sm:min-w-[280px]"
        />
      </div>
      <p className="text-xs text-muted-3">{cliSubtext}</p>
    </div>
  );
}

function PrimaryButton({
  platform,
  arch,
  primaryLabel,
  comingSoonLabel,
  desktopOnlyLabel,
}: {
  platform: Platform;
  arch: MacArch;
  primaryLabel: string;
  comingSoonLabel: string;
  desktopOnlyLabel: string;
}) {
  // SSR / first paint / browser we couldn't classify: render a
  // skeleton-ish neutral pill instead of flashing a wrong CTA.
  // Same height/width as the macOS button to avoid layout shift.
  if (platform === "unknown" || platform === "other") {
    return (
      <span
        aria-hidden="true"
        className="inline-flex h-12 w-[180px] animate-pulse items-center justify-center rounded-full bg-surface-muted text-sm text-muted-3"
      />
    );
  }

  if (platform === "macos") {
    // Apple Silicon (arm64) gets the M-series DMG. Intel users get
    // the x86_64 DMG. If we couldn't detect arch (Safari without
    // WebGL hints) we fall back to arm64 — most macOS users are on
    // Apple Silicon as of 2026, and Rosetta lets the arm64 binary
    // run on Intel anyway (slower, but it launches).
    const href = arch === "intel" ? MACOS_X64_URL : MACOS_ARM64_URL;
    const labelSuffix =
      arch === "intel"
        ? " (Intel)"
        : arch === "arm64"
          ? " (Apple Silicon)"
          : "";
    return (
      <a
        href={href}
        rel="noreferrer"
        className="inline-flex h-12 items-center justify-center gap-2 rounded-full bg-inverse px-6 text-sm font-medium text-on-inverse transition hover:bg-inverse-hover"
      >
        {primaryLabel}
        {labelSuffix ? (
          <span className="ml-1 text-xs opacity-75">{labelSuffix}</span>
        ) : null}
        <ArrowRight className="size-4" />
      </a>
    );
  }

  // Linux / Windows — binary not yet self-contained (sidecar bundling pending).
  if (platform === "linux" || platform === "windows") {
    return (
      <span
        aria-disabled="true"
        className="inline-flex h-12 cursor-not-allowed items-center justify-center gap-2 rounded-full border border-border-base bg-surface-muted px-6 text-sm font-medium text-muted-2"
      >
        <Clock className="size-4" />
        {comingSoonLabel.replace(
          "{os}",
          platform === "windows" ? "Windows" : "Linux",
        )}
      </span>
    );
  }

  // Mobile + iPad — desktop app fundamentally won't run here.
  return (
    <span
      aria-disabled="true"
      className="inline-flex h-12 cursor-not-allowed items-center justify-center gap-2 rounded-full border border-border-base bg-surface-muted px-6 text-sm font-medium text-muted-2"
    >
      <Clock className="size-4" />
      {desktopOnlyLabel}
    </span>
  );
}
